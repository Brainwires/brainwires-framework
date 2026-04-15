/**
 * Tests for OpenAPI tool generation and execution.
 */

import { assertEquals, assertExists } from "@std/assert";
import {
  executeOpenApiToolWithEndpoint,
  openApiToToolDefs,
  openApiToTools,
} from "./openapi.ts";
import type { OpenApiEndpoint } from "./openapi.ts";

function petstoreSpec(): Record<string, unknown> {
  return {
    openapi: "3.0.0",
    info: { title: "Petstore", version: "1.0.0" },
    servers: [{ url: "https://petstore.example.com/v1" }],
    paths: {
      "/pets": {
        get: {
          operationId: "listPets",
          summary: "List all pets",
          parameters: [
            {
              name: "limit",
              in: "query",
              required: false,
              schema: { type: "integer" },
              description: "How many items to return",
            },
          ],
          responses: { "200": { description: "OK" } },
        },
        post: {
          operationId: "createPet",
          summary: "Create a pet",
          requestBody: {
            required: true,
            content: {
              "application/json": {
                schema: {
                  type: "object",
                  properties: {
                    name: { type: "string" },
                    tag: { type: "string" },
                  },
                },
              },
            },
          },
          responses: { "201": { description: "Created" } },
        },
      },
      "/pets/{petId}": {
        get: {
          operationId: "showPetById",
          summary: "Info for a specific pet",
          parameters: [
            {
              name: "petId",
              in: "path",
              required: true,
              schema: { type: "string" },
              description: "The id of the pet",
            },
          ],
          responses: { "200": { description: "OK" } },
        },
      },
    },
  };
}

Deno.test("openApiToTools - parse petstore spec", () => {
  const tools = openApiToTools(petstoreSpec());
  assertEquals(tools.length, 3);

  const list = tools.find((t) => t.name === "listPets");
  assertExists(list);
  assertEquals(list.description, "List all pets");
  assertExists(list.input_schema.properties?.limit);
  assertEquals(list.input_schema.properties!.limit.type, "integer");
  // limit is not required
  assertEquals(list.input_schema.required, undefined);

  const create = tools.find((t) => t.name === "createPet");
  assertExists(create);
  assertEquals(create.description, "Create a pet");
  assertExists(create.input_schema.properties?.body);

  const show = tools.find((t) => t.name === "showPetById");
  assertExists(show);
  assertEquals(show.input_schema.required, ["petId"]);
});

Deno.test("openApiToToolDefs - endpoint details", () => {
  const defs = openApiToToolDefs(petstoreSpec());
  assertEquals(defs.length, 3);

  const list = defs.find((d) => d.tool.name === "listPets")!;
  assertEquals(list.endpoint.method, "GET");
  assertEquals(list.endpoint.path, "/pets");
  assertEquals(list.endpoint.baseUrl, "https://petstore.example.com/v1");
  assertEquals(list.endpoint.queryParams.length, 1);
  assertEquals(list.endpoint.queryParams[0].name, "limit");
  assertEquals(list.endpoint.queryParams[0].required, false);

  const create = defs.find((d) => d.tool.name === "createPet")!;
  assertEquals(create.endpoint.method, "POST");
  assertEquals(create.endpoint.hasBody, true);

  const show = defs.find((d) => d.tool.name === "showPetById")!;
  assertEquals(show.endpoint.method, "GET");
  assertEquals(show.endpoint.pathParams.length, 1);
  assertEquals(show.endpoint.pathParams[0].name, "petId");
  assertEquals(show.endpoint.pathParams[0].required, true);
});

Deno.test("openApiToTools - operation id fallback", () => {
  const spec = {
    openapi: "3.0.0",
    info: { title: "Test", version: "1.0.0" },
    servers: [{ url: "https://api.example.com" }],
    paths: {
      "/users/{id}/posts": {
        get: {
          summary: "Get user posts",
          responses: { "200": { description: "OK" } },
        },
      },
    },
  };

  const tools = openApiToTools(spec);
  assertEquals(tools.length, 1);
  assertEquals(tools[0].name, "get_users_id_posts");
});

Deno.test("openApiToTools - empty spec", () => {
  const spec = {
    openapi: "3.0.0",
    info: { title: "Empty", version: "1.0.0" },
    paths: {},
  };

  const tools = openApiToTools(spec);
  assertEquals(tools.length, 0);
});

Deno.test("openApiToTools - invalid spec throws", () => {
  let threw = false;
  try {
    openApiToTools(null);
  } catch {
    threw = true;
  }
  assertEquals(threw, true);
});

Deno.test("executeOpenApiToolWithEndpoint - URL building with path params", async () => {
  const endpoint: OpenApiEndpoint = {
    method: "GET",
    path: "/pets/{petId}",
    baseUrl: "https://petstore.example.com/v1",
    pathParams: [
      { name: "petId", required: true, schemaType: "string" },
    ],
    queryParams: [],
    headerParams: [],
    hasBody: false,
  };

  // We can't actually fetch, but we can verify error for missing params
  const result = await executeOpenApiToolWithEndpoint(endpoint, {});
  assertEquals(result.is_error, true);
  assertEquals(result.content.includes("Missing required path parameter"), true);
});

Deno.test("executeOpenApiToolWithEndpoint - missing required query param", async () => {
  const endpoint: OpenApiEndpoint = {
    method: "GET",
    path: "/search",
    baseUrl: "https://api.example.com",
    pathParams: [],
    queryParams: [
      { name: "q", required: true, schemaType: "string" },
    ],
    headerParams: [],
    hasBody: false,
  };

  const result = await executeOpenApiToolWithEndpoint(endpoint, {});
  assertEquals(result.is_error, true);
  assertEquals(result.content.includes("Missing required query parameter"), true);
});

Deno.test("openApiToTools - path-level parameters", () => {
  const spec = {
    openapi: "3.0.0",
    info: { title: "Test", version: "1.0.0" },
    paths: {
      "/items/{itemId}": {
        parameters: [
          {
            name: "itemId",
            in: "path",
            required: true,
            schema: { type: "string" },
          },
        ],
        get: {
          operationId: "getItem",
          summary: "Get item",
          responses: { "200": { description: "OK" } },
        },
        delete: {
          operationId: "deleteItem",
          summary: "Delete item",
          responses: { "204": { description: "No content" } },
        },
      },
    },
  };

  const tools = openApiToTools(spec);
  assertEquals(tools.length, 2);

  // Both operations should inherit the path-level param
  for (const tool of tools) {
    assertExists(tool.input_schema.properties?.itemId);
    assertEquals(tool.input_schema.required?.includes("itemId"), true);
  }
});
