/**
 * AgentCard construction and serialization tests.
 */

import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import type {
  AgentCard,
  AgentCapabilities,
  AgentExtension,
  AgentInterface,
  AgentProvider,
  AgentSkill,
  ApiKeySecurityScheme,
  AuthorizationCodeFlow,
  ClientCredentialsFlow,
  DeviceCodeFlow,
  HttpSecurityScheme,
  MutualTlsSecurityScheme,
  OAuth2SecurityScheme,
  OAuthFlows,
  OpenIdConnectSecurityScheme,
  SecurityRequirement,
  SecurityScheme,
} from "./agent_card.ts";

function minimalAgentCard(): AgentCard {
  return {
    name: "Test Agent",
    description: "A test agent",
    version: "1.0.0",
    capabilities: {},
    skills: [],
    defaultInputModes: ["text/plain"],
    defaultOutputModes: ["text/plain"],
  };
}

Deno.test("AgentCard minimal round-trip", () => {
  const card = minimalAgentCard();
  const parsed = JSON.parse(JSON.stringify(card)) as AgentCard;
  assertEquals(parsed.name, "Test Agent");
  assertEquals(parsed.version, "1.0.0");
  assertEquals(parsed.defaultInputModes, ["text/plain"]);
  assertEquals(parsed.skills.length, 0);
});

Deno.test("AgentCard with full fields", () => {
  const card: AgentCard = {
    ...minimalAgentCard(),
    supportedInterfaces: [
      {
        url: "https://agent.example.com/a2a",
        protocolBinding: "JSONRPC",
        protocolVersion: "0.2.1",
      },
    ],
    capabilities: {
      streaming: true,
      pushNotifications: false,
      extendedAgentCard: true,
      extensions: [
        {
          uri: "ext://custom",
          description: "Custom extension",
          required: false,
        },
      ],
    },
    skills: [
      {
        id: "summarize",
        name: "Summarize",
        description: "Summarizes text",
        tags: ["nlp", "summarization"],
        examples: ["Summarize this article"],
        inputModes: ["text/plain"],
        outputModes: ["text/plain"],
      },
    ],
    provider: {
      url: "https://example.com",
      organization: "Example Corp",
    },
    documentationUrl: "https://docs.example.com",
    iconUrl: "https://example.com/icon.png",
    signatures: [
      {
        protected: "eyJhbGciOiJFUzI1NiJ9",
        signature: "abc123",
        header: { kid: "key-1" },
      },
    ],
  };

  const parsed = JSON.parse(JSON.stringify(card)) as AgentCard;
  assertEquals(parsed.supportedInterfaces?.length, 1);
  assertEquals(parsed.supportedInterfaces![0].protocolBinding, "JSONRPC");
  assertEquals(parsed.capabilities.streaming, true);
  assertEquals(parsed.capabilities.extensions?.length, 1);
  assertEquals(parsed.skills.length, 1);
  assertEquals(parsed.skills[0].id, "summarize");
  assertEquals(parsed.provider?.organization, "Example Corp");
  assertEquals(parsed.signatures?.length, 1);
});

Deno.test("AgentInterface serializes with camelCase", () => {
  const iface: AgentInterface = {
    url: "https://agent.example.com",
    protocolBinding: "HTTP+JSON",
    tenant: "tenant-1",
    protocolVersion: "0.2.1",
  };
  const json = JSON.stringify(iface);
  const obj = JSON.parse(json);
  assertEquals(obj.protocolBinding, "HTTP+JSON");
  assertEquals(obj.protocolVersion, "0.2.1");
  assertEquals(obj.tenant, "tenant-1");
});

Deno.test("SecurityScheme apiKey variant", () => {
  const scheme: ApiKeySecurityScheme = {
    type: "apiKey",
    name: "X-API-Key",
    in: "header",
    description: "API key auth",
  };
  const parsed = JSON.parse(JSON.stringify(scheme)) as SecurityScheme;
  assertEquals(parsed.type, "apiKey");
  assertEquals((parsed as ApiKeySecurityScheme).name, "X-API-Key");
  assertEquals((parsed as ApiKeySecurityScheme).in, "header");
});

Deno.test("SecurityScheme http variant", () => {
  const scheme: HttpSecurityScheme = {
    type: "http",
    scheme: "Bearer",
    bearerFormat: "JWT",
  };
  const parsed = JSON.parse(JSON.stringify(scheme)) as SecurityScheme;
  assertEquals(parsed.type, "http");
  assertEquals((parsed as HttpSecurityScheme).scheme, "Bearer");
  assertEquals((parsed as HttpSecurityScheme).bearerFormat, "JWT");
});

Deno.test("SecurityScheme oauth2 with authorizationCode flow", () => {
  const flow: AuthorizationCodeFlow = {
    type: "authorizationCode",
    authorizationUrl: "https://auth.example.com/authorize",
    tokenUrl: "https://auth.example.com/token",
    refreshUrl: "https://auth.example.com/refresh",
    scopes: { read: "Read access", write: "Write access" },
    pkceRequired: true,
  };
  const scheme: OAuth2SecurityScheme = {
    type: "oauth2",
    flows: flow,
    oauth2MetadataUrl: "https://auth.example.com/.well-known/openid-configuration",
  };
  const parsed = JSON.parse(JSON.stringify(scheme)) as OAuth2SecurityScheme;
  assertEquals(parsed.type, "oauth2");
  assertEquals(parsed.flows.type, "authorizationCode");
  const parsedFlow = parsed.flows as AuthorizationCodeFlow;
  assertEquals(parsedFlow.authorizationUrl, "https://auth.example.com/authorize");
  assertEquals(parsedFlow.pkceRequired, true);
  assertEquals(parsedFlow.scopes.read, "Read access");
});

Deno.test("SecurityScheme oauth2 with clientCredentials flow", () => {
  const flow: ClientCredentialsFlow = {
    type: "clientCredentials",
    tokenUrl: "https://auth.example.com/token",
    scopes: { api: "API access" },
  };
  const scheme: OAuth2SecurityScheme = { type: "oauth2", flows: flow };
  const parsed = JSON.parse(JSON.stringify(scheme)) as OAuth2SecurityScheme;
  assertEquals(parsed.flows.type, "clientCredentials");
});

Deno.test("SecurityScheme oauth2 with deviceCode flow", () => {
  const flow: DeviceCodeFlow = {
    type: "deviceCode",
    deviceAuthorizationUrl: "https://auth.example.com/device",
    tokenUrl: "https://auth.example.com/token",
    scopes: {},
  };
  const scheme: OAuth2SecurityScheme = { type: "oauth2", flows: flow };
  const parsed = JSON.parse(JSON.stringify(scheme)) as OAuth2SecurityScheme;
  assertEquals((parsed.flows as DeviceCodeFlow).deviceAuthorizationUrl, "https://auth.example.com/device");
});

Deno.test("SecurityScheme openIdConnect variant", () => {
  const scheme: OpenIdConnectSecurityScheme = {
    type: "openIdConnect",
    openIdConnectUrl: "https://auth.example.com/.well-known/openid-configuration",
  };
  const parsed = JSON.parse(JSON.stringify(scheme)) as SecurityScheme;
  assertEquals(parsed.type, "openIdConnect");
});

Deno.test("SecurityScheme mutualTls variant", () => {
  const scheme: MutualTlsSecurityScheme = {
    type: "mutualTls",
    description: "mTLS auth",
  };
  const parsed = JSON.parse(JSON.stringify(scheme)) as SecurityScheme;
  assertEquals(parsed.type, "mutualTls");
});

Deno.test("SecurityRequirement round-trips", () => {
  const req: SecurityRequirement = {
    schemes: { oauth2: ["read", "write"], apiKey: [] },
  };
  const parsed = JSON.parse(JSON.stringify(req)) as SecurityRequirement;
  assertEquals(parsed.schemes.oauth2, ["read", "write"]);
  assertEquals(parsed.schemes.apiKey, []);
});

Deno.test("AgentCard with security schemes", () => {
  const card: AgentCard = {
    ...minimalAgentCard(),
    securitySchemes: {
      bearer: { type: "http", scheme: "Bearer" },
      apiKey: { type: "apiKey", name: "key", in: "header" },
    },
    securityRequirements: [
      { schemes: { bearer: [] } },
    ],
  };
  const parsed = JSON.parse(JSON.stringify(card)) as AgentCard;
  assertEquals(Object.keys(parsed.securitySchemes!).length, 2);
  assertEquals(parsed.securitySchemes!.bearer.type, "http");
  assertEquals(parsed.securityRequirements!.length, 1);
});

Deno.test("AgentSkill with all fields", () => {
  const skill: AgentSkill = {
    id: "translate",
    name: "Translate",
    description: "Translates between languages",
    tags: ["nlp", "translation"],
    examples: ["Translate 'hello' to French"],
    inputModes: ["text/plain"],
    outputModes: ["text/plain"],
    securityRequirements: [{ schemes: { bearer: [] } }],
  };
  const parsed = JSON.parse(JSON.stringify(skill)) as AgentSkill;
  assertEquals(parsed.id, "translate");
  assertEquals(parsed.tags.length, 2);
  assertEquals(parsed.securityRequirements?.length, 1);
});
