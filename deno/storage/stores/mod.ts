/**
 * Domain stores for conversation, message, task, plan, and template data.
 * @module
 */

export {
  type MessageMetadata,
  type MessageStoreI,
  MessageStore,
  InMemoryMessageStore,
} from "./message_store.ts";

export {
  type ConversationMetadata,
  type ConversationStoreI,
  ConversationStore,
  InMemoryConversationStore,
} from "./conversation_store.ts";

export {
  type TaskMetadata,
  type AgentStateMetadata,
  type TaskStoreI,
  type AgentStateStoreI,
  TaskStore,
  InMemoryTaskStore,
  AgentStateStore,
  InMemoryAgentStateStore,
  taskToMetadata,
  metadataToTask,
} from "./task_store.ts";

export {
  type PlanStoreI,
  PlanStore,
  InMemoryPlanStore,
} from "./plan_store.ts";

export {
  type PlanTemplate,
  TemplateStore,
  createTemplate,
  createTemplateFromPlan,
  withCategory,
  withTags,
  instantiateTemplate,
  extractVariables,
  markUsed,
} from "./template_store.ts";
