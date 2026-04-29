// brainwires-chat-pwa — WebRTC transport to the home daemon.
//
// Owns one RTCPeerConnection plus the "a2a" data channel. Drives the
// browser side of the SDP offer/answer + ICE-trickle handshake against a
// SignalingClient, then exposes a tiny request/response API on top of the
// data channel using JSON-RPC ids.
//
// State machine:
//   'idle'       — constructed, no connect() yet
//   'connecting' — connect() in flight
//   'connected'  — data channel open
//   'closing'    — close() in flight
//   'closed'     — close() done
//   'failed'     — ICE failed / dc closed unexpectedly
//
// Wire format on the channel: a single JSON-RPC envelope per text frame.
// Length-prefixed binary framing is reserved for M11. ICE-restart and
// the heartbeat ping are deferred to M10.

// SignalingClient is the expected shape of opts.signaling — see ./home-signaling.js.
// We don't import it here to avoid a circular-looking edge for tooling; the
// JSDoc reference is informational.

/**
 * JsonRpcDispatcher — the pure piece of HomeTransport's request bookkeeping.
 *
 * Allocates monotonic ids, parks a Promise for each outstanding request, and
 * dispatches inbound replies back to the right caller. Broken out so the
 * id-allocation + reply-routing logic can be unit-tested without an
 * RTCPeerConnection in scope (Node `--test` has no RTC).
 */
export class JsonRpcDispatcher {
    constructor() {
        this._nextId = 1;
        this._pending = new Map(); // id -> { resolve, reject, timer }
    }

    /** Allocate a fresh id and park a Promise; returns { id, frame, promise }. */
    request(method, params, { timeoutMs = 30000 } = {}) {
        const id = this._nextId++;
        const frame = JSON.stringify({ jsonrpc: '2.0', id, method, params });
        const promise = new Promise((resolve, reject) => {
            const timer = (timeoutMs > 0 && typeof setTimeout === 'function')
                ? setTimeout(() => {
                    if (this._pending.delete(id)) {
                        reject(new Error(`jsonrpc: request '${method}' (id=${id}) timed out after ${timeoutMs}ms`));
                    }
                }, timeoutMs)
                : null;
            this._pending.set(id, { resolve, reject, timer });
        });
        return { id, frame, promise };
    }

    /**
     * Route an inbound text frame. Resolves/rejects the matching pending
     * promise. Unknown ids (server-push, stray notifications) are dropped.
     * Returns true if the frame matched a pending request, false otherwise.
     */
    dispatch(text) {
        let msg;
        try { msg = JSON.parse(text); }
        catch (_) { return false; }
        if (!msg || typeof msg !== 'object') return false;
        // Only request/response replies have an id; notifications don't.
        if (msg.id == null) return false;
        const slot = this._pending.get(msg.id);
        if (!slot) return false;
        this._pending.delete(msg.id);
        if (slot.timer) clearTimeout(slot.timer);
        if (msg.error) {
            const e = new Error(msg.error.message || 'jsonrpc error');
            e.code = msg.error.code;
            e.data = msg.error.data;
            slot.reject(e);
        } else {
            slot.resolve(msg.result);
        }
        return true;
    }

    /** Reject everything still pending — used on close()/failure. */
    rejectAll(reason) {
        const err = reason instanceof Error ? reason : new Error(String(reason));
        for (const slot of this._pending.values()) {
            if (slot.timer) clearTimeout(slot.timer);
            slot.reject(err);
        }
        this._pending.clear();
    }

    get pendingCount() { return this._pending.size; }
}

/**
 * HomeTransport — owns one RTCPeerConnection + 'a2a' data channel.
 *
 * Browser-only at runtime. The Node test suite exercises JsonRpcDispatcher
 * directly; the full handshake is covered by the M3 daemon's webrtc.rs test
 * and the eventual chat-PWA e2e suite (M9+).
 */
export class HomeTransport {
    /**
     * @param {{
     *   signaling: import('./home-signaling.js').SignalingClient,
     *   iceServers?: RTCIceServer[],
     *   rtcPeerConnection?: typeof RTCPeerConnection,
     * }} opts
     */
    constructor(opts) {
        if (!opts || !opts.signaling) {
            throw new Error('HomeTransport: signaling is required');
        }
        this._signaling = opts.signaling;
        this._iceServers = Array.isArray(opts.iceServers) ? opts.iceServers : null;
        this._RTC = opts.rtcPeerConnection || (typeof RTCPeerConnection !== 'undefined' ? RTCPeerConnection : null);

        this._sessionId = null;
        this._pc = null;
        this._dc = null;
        this._state = 'idle';
        this._dispatcher = new JsonRpcDispatcher();
        this._iceAbort = null;        // AbortController for the inbound ICE long-poll
        this._iceCursor = 0;
        this._inboundIcePump = null;  // Promise for the running ICE pump task
        this._connectResolve = null;
        this._connectReject = null;
    }

    get sessionId() { return this._sessionId; }
    get state() { return this._state; }

    _setState(next) { this._state = next; }

    /**
     * Begin the connection. Resolves when the data channel is open.
     */
    async connect() {
        if (this._state !== 'idle') {
            throw new Error(`HomeTransport.connect: already ${this._state}`);
        }
        if (!this._RTC) {
            throw new Error('HomeTransport: RTCPeerConnection unavailable in this environment');
        }
        this._setState('connecting');

        try {
            // 1. Create signaling session — server returns an ICE-server hint list.
            const session = await this._signaling.createSession();
            this._sessionId = session.session_id;
            const iceServers = this._iceServers || (Array.isArray(session.ice_servers) ? session.ice_servers : []);

            // 2. Build the peer connection + 'a2a' data channel BEFORE createOffer.
            //    (Otherwise the channel won't appear in the SDP and the home
            //    side never sees an `ondatachannel` event.)
            const pc = new this._RTC({ iceServers });
            this._pc = pc;
            const dc = pc.createDataChannel('a2a');
            this._dc = dc;

            // 3. Arm the data channel listeners early so we don't miss the
            //    'open' event in case it fires before we await it below.
            const dcOpen = new Promise((resolve, reject) => {
                dc.onopen = () => resolve();
                dc.onerror = (e) => reject(new Error(`a2a data channel error: ${e && e.message ? e.message : 'unknown'}`));
                dc.onclose = () => {
                    if (this._state === 'connected') {
                        this._setState('failed');
                        this._dispatcher.rejectAll(new Error('a2a data channel closed unexpectedly'));
                    }
                };
                dc.onmessage = (ev) => {
                    if (typeof ev.data === 'string') {
                        const matched = this._dispatcher.dispatch(ev.data);
                        if (!matched) console.debug('home-transport: dropped unmatched frame', ev.data);
                    } else {
                        // M5 only sends/receives text frames. Binary framing is M11.
                        console.debug('home-transport: ignoring non-text frame');
                    }
                };
            });

            // 4. Outbound ICE: forward each local candidate to the daemon.
            //    Fire-and-forget; an error here logs but doesn't fail the
            //    handshake (we may still succeed with already-trickled cands).
            pc.onicecandidate = (ev) => {
                if (!ev.candidate) return;
                const c = ev.candidate;
                this._signaling.postIce(
                    this._sessionId,
                    c.candidate,
                    c.sdpMid,
                    c.sdpMLineIndex,
                ).catch((e) => console.warn('home-transport: postIce failed:', e && e.message ? e.message : e));
            };

            pc.oniceconnectionstatechange = () => {
                const s = pc.iceConnectionState;
                if (s === 'failed') {
                    this._setState('failed');
                    this._dispatcher.rejectAll(new Error('ICE connection failed'));
                }
            };

            // 5. createOffer → setLocalDescription → POST /signal/offer.
            const offer = await pc.createOffer();
            await pc.setLocalDescription(offer);
            await this._signaling.postOffer(this._sessionId, offer.sdp);

            // 6. Pull the answer (the home daemon stashes it before the POST
            //    returns 204, so this should fast-path).
            let answer = null;
            for (let attempt = 0; attempt < 4 && !answer; attempt++) {
                answer = await this._signaling.pollAnswer(this._sessionId);
            }
            if (!answer) throw new Error('HomeTransport.connect: no SDP answer received');
            await pc.setRemoteDescription({ type: 'answer', sdp: answer.sdp });

            // 7. Inbound ICE pump: long-poll the daemon for its trickled
            //    candidates and addIceCandidate them.
            this._iceAbort = new AbortController();
            this._inboundIcePump = this._runIcePump(this._iceAbort.signal).catch((e) => {
                if (e && e.name === 'AbortError') return;
                console.warn('home-transport: ICE pump exited:', e && e.message ? e.message : e);
            });

            // 8. Wait for the data channel to open.
            await dcOpen;
            this._setState('connected');
        } catch (err) {
            this._setState('failed');
            // Best-effort cleanup of the signaling session and PC.
            if (this._iceAbort) try { this._iceAbort.abort(); } catch (_) {}
            if (this._pc) try { this._pc.close(); } catch (_) {}
            if (this._sessionId) try { await this._signaling.closeSession(this._sessionId); } catch (_) {}
            throw err;
        }
    }

    async _runIcePump(signal) {
        // Long-poll loop. The daemon returns 204 on timeout; pollIce maps
        // that to {candidates:[], cursor:since}. We keep going until the
        // session is closed or we hit a hard error (404 from the server,
        // which we treat as "session gone").
        while (!signal.aborted && (this._state === 'connecting' || this._state === 'connected')) {
            let next;
            try {
                next = await this._signaling.pollIce(this._sessionId, this._iceCursor, signal);
            } catch (e) {
                if (e && e.name === 'AbortError') return;
                // 404 (session gone) → bail; transient network errors → bail too,
                // M10 will add reconnect.
                throw e;
            }
            for (const c of next.candidates) {
                if (!c || c.candidate == null) continue;
                try {
                    await this._pc.addIceCandidate({
                        candidate: c.candidate,
                        sdpMid: c.sdp_mid ?? undefined,
                        sdpMLineIndex: typeof c.sdp_m_line_index === 'number' ? c.sdp_m_line_index : undefined,
                    });
                } catch (e) {
                    console.warn('home-transport: addIceCandidate failed:', e && e.message ? e.message : e);
                }
            }
            this._iceCursor = next.cursor;
        }
    }

    /**
     * Send a JSON-RPC request. Resolves with the result; rejects on error
     * reply or timeout.
     * @param {string} method
     * @param {object} [params]
     * @param {{timeoutMs?: number}} [opts]
     */
    async request(method, params, { timeoutMs = 30000 } = {}) {
        if (this._state !== 'connected') {
            throw new Error(`HomeTransport.request: not connected (state=${this._state})`);
        }
        const { frame, promise } = this._dispatcher.request(method, params, { timeoutMs });
        try {
            this._dc.send(frame);
        } catch (e) {
            // Drop the pending slot so we don't leak; timeout would also
            // catch this, but failing fast is friendlier.
            this._dispatcher.rejectAll(new Error(`jsonrpc: send failed: ${e && e.message ? e.message : e}`));
            throw e;
        }
        return await promise;
    }

    /** Disconnect cleanly. Idempotent. */
    async close() {
        if (this._state === 'closed' || this._state === 'closing') return;
        this._setState('closing');
        if (this._iceAbort) { try { this._iceAbort.abort(); } catch (_) {} }
        this._dispatcher.rejectAll(new Error('HomeTransport closed'));
        try { if (this._dc) this._dc.close(); } catch (_) {}
        try { if (this._pc) this._pc.close(); } catch (_) {}
        if (this._sessionId) {
            try { await this._signaling.closeSession(this._sessionId); } catch (_) {}
        }
        this._setState('closed');
    }
}
