/**
 * Domain stores for conversation, message, task, plan, and template data.
 * @module
 */

export {
  InMemoryMessageStore,
  type MessageMetadata,
  MessageStore,
  type MessageStoreI,
} from "./message_store.ts";

export {
  type ConversationMetadata,
  ConversationStore,
  type ConversationStoreI,
  InMemoryConversationStore,
} from "./conversation_store.ts";

export {
  type AgentStateMetadata,
  AgentStateStore,
  type AgentStateStoreI,
  InMemoryAgentStateStore,
  InMemoryTaskStore,
  metadataToTask,
  type TaskMetadata,
  TaskStore,
  type TaskStoreI,
  taskToMetadata,
} from "./task_store.ts";

export { InMemoryPlanStore, PlanStore, type PlanStoreI } from "./plan_store.ts";

export {
  createTemplate,
  createTemplateFromPlan,
  extractVariables,
  instantiateTemplate,
  markUsed,
  type PlanTemplate,
  TemplateStore,
  withCategory,
  withTags,
} from "./template_store.ts";
