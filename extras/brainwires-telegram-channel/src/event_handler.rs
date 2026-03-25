//! Teloxide dispatcher that converts Telegram updates to `ChannelEvent`.

use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::respond;
use teloxide::types::Message;
use tokio::sync::mpsc;

use brainwires_channels::ChannelEvent;

use crate::telegram::telegram_message_to_channel_message;

/// Starts the teloxide update dispatcher, forwarding events over the provided sender.
///
/// This function blocks until the bot is shut down.
pub async fn run_dispatcher(bot: Bot, event_tx: mpsc::Sender<ChannelEvent>) {
    let event_tx = Arc::new(event_tx);

    let message_handler = {
        let tx = Arc::clone(&event_tx);
        Update::filter_message().endpoint(move |msg: Message| {
            let tx = Arc::clone(&tx);
            async move {
                // Skip bot messages to avoid loops
                if let Some(ref from) = msg.from {
                    if from.is_bot {
                        return respond(());
                    }
                }

                let channel_message = telegram_message_to_channel_message(&msg);
                let event = ChannelEvent::MessageReceived(channel_message);

                if let Err(e) = tx.send(event).await {
                    tracing::error!("Failed to forward message event: {}", e);
                }

                respond(())
            }
        })
    };

    let edited_message_handler = {
        let tx = Arc::clone(&event_tx);
        Update::filter_edited_message().endpoint(move |msg: Message| {
            let tx = Arc::clone(&tx);
            async move {
                let channel_message = telegram_message_to_channel_message(&msg);
                let event = ChannelEvent::MessageEdited(channel_message);

                if let Err(e) = tx.send(event).await {
                    tracing::error!("Failed to forward edited message event: {}", e);
                }

                respond(())
            }
        })
    };

    let handler = dptree::entry()
        .branch(message_handler)
        .branch(edited_message_handler);

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
