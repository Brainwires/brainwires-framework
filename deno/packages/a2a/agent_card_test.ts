/**
 * AgentCard construction and serialization tests (v1.0).
 */

import { assertEquals } from "@std/assert";
import type {
  AgentCard,
  AgentInterface,
  AgentSkill,
  AuthorizationCodeOAuthFlow,
  ClientCredentialsOAuthFlow,
  DeviceCodeOAuthFlow,
  SecurityRequirement,
  SecurityScheme,
} from "./agent_card.ts";

function minimalAgentCard(): AgentCard {
  return {
    name: "Test Agent",
    description: "A test agent",
    version: "1.0.0",
    supportedInterfaces: [],
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
  assertEquals(parsed.supportedInterfaces.length, 0);
});

Deno.test("AgentCard with full fields", () => {
  const card: AgentCard = {
    ...minimalAgentCard(),
    supportedInterfaces: [
      {
        url: "https://agent.example.com/a2a",
        protocolBinding: "JSONRPC",
        protocolVersion: "1.0",
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
  assertEquals(parsed.supportedInterfaces.length, 1);
  assertEquals(parsed.supportedInterfaces[0].protocolBinding, "JSONRPC");
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
    protocolVersion: "1.0",
  };
  const json = JSON.stringify(iface);
  const obj = JSON.parse(json);
  assertEquals(obj.protocolBinding, "HTTP+JSON");
  assertEquals(obj.protocolVersion, "1.0");
  assertEquals(obj.tenant, "tenant-1");
});

Deno.test("SecurityScheme apiKey wrapper variant", () => {
  const scheme: SecurityScheme = {
    apiKeySecurityScheme: {
      name: "X-API-Key",
      in: "header",
      description: "API key auth",
    },
  };
  const parsed = JSON.parse(JSON.stringify(scheme)) as SecurityScheme;
  assertEquals(parsed.apiKeySecurityScheme?.name, "X-API-Key");
  assertEquals(parsed.apiKeySecurityScheme?.in, "header");
});

Deno.test("SecurityScheme httpAuth wrapper variant", () => {
  const scheme: SecurityScheme = {
    httpAuthSecurityScheme: {
      scheme: "Bearer",
      bearerFormat: "JWT",
    },
  };
  const parsed = JSON.parse(JSON.stringify(scheme)) as SecurityScheme;
  assertEquals(parsed.httpAuthSecurityScheme?.scheme, "Bearer");
  assertEquals(parsed.httpAuthSecurityScheme?.bearerFormat, "JWT");
});

Deno.test("SecurityScheme oauth2 with authorizationCode flow", () => {
  const flow: AuthorizationCodeOAuthFlow = {
    authorizationUrl: "https://auth.example.com/authorize",
    tokenUrl: "https://auth.example.com/token",
    refreshUrl: "https://auth.example.com/refresh",
    scopes: { read: "Read access", write: "Write access" },
    pkceRequired: true,
  };
  const scheme: SecurityScheme = {
    oauth2SecurityScheme: {
      flows: { authorizationCode: flow },
      oauth2MetadataUrl: "https://auth.example.com/.well-known/openid-configuration",
    },
  };
  const parsed = JSON.parse(JSON.stringify(scheme)) as SecurityScheme;
  const oauth2 = parsed.oauth2SecurityScheme!;
  assertEquals(oauth2.flows.authorizationCode?.authorizationUrl, "https://auth.example.com/authorize");
  assertEquals(oauth2.flows.authorizationCode?.pkceRequired, true);
  assertEquals(oauth2.flows.authorizationCode?.scopes.read, "Read access");
});

Deno.test("SecurityScheme oauth2 with clientCredentials flow", () => {
  const flow: ClientCredentialsOAuthFlow = {
    tokenUrl: "https://auth.example.com/token",
    scopes: { api: "API access" },
  };
  const scheme: SecurityScheme = {
    oauth2SecurityScheme: {
      flows: { clientCredentials: flow },
    },
  };
  const parsed = JSON.parse(JSON.stringify(scheme)) as SecurityScheme;
  assertEquals(parsed.oauth2SecurityScheme?.flows.clientCredentials?.tokenUrl, "https://auth.example.com/token");
});

Deno.test("SecurityScheme oauth2 with deviceCode flow", () => {
  const flow: DeviceCodeOAuthFlow = {
    deviceAuthorizationUrl: "https://auth.example.com/device",
    tokenUrl: "https://auth.example.com/token",
    scopes: {},
  };
  const scheme: SecurityScheme = {
    oauth2SecurityScheme: {
      flows: { deviceCode: flow },
    },
  };
  const parsed = JSON.parse(JSON.stringify(scheme)) as SecurityScheme;
  assertEquals(parsed.oauth2SecurityScheme?.flows.deviceCode?.deviceAuthorizationUrl, "https://auth.example.com/device");
});

Deno.test("SecurityScheme openIdConnect wrapper variant", () => {
  const scheme: SecurityScheme = {
    openIdConnectSecurityScheme: {
      openIdConnectUrl: "https://auth.example.com/.well-known/openid-configuration",
    },
  };
  const parsed = JSON.parse(JSON.stringify(scheme)) as SecurityScheme;
  assertEquals(parsed.openIdConnectSecurityScheme?.openIdConnectUrl, "https://auth.example.com/.well-known/openid-configuration");
});

Deno.test("SecurityScheme mutualTls wrapper variant", () => {
  const scheme: SecurityScheme = {
    mtlsSecurityScheme: {
      description: "mTLS auth",
    },
  };
  const parsed = JSON.parse(JSON.stringify(scheme)) as SecurityScheme;
  assertEquals(parsed.mtlsSecurityScheme?.description, "mTLS auth");
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
      bearer: { httpAuthSecurityScheme: { scheme: "Bearer" } },
      apiKey: { apiKeySecurityScheme: { name: "key", in: "header" } },
    },
    securityRequirements: [
      { schemes: { bearer: [] } },
    ],
  };
  const parsed = JSON.parse(JSON.stringify(card)) as AgentCard;
  assertEquals(Object.keys(parsed.securitySchemes!).length, 2);
  assertEquals(parsed.securitySchemes!.bearer.httpAuthSecurityScheme?.scheme, "Bearer");
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
