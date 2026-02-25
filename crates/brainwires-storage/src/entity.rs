//! Entity Types for Knowledge Graph
//!
//! Types for representing extracted entities and their relationships
//! in conversation memory. Used by the relationship graph and knowledge system.

use std::collections::{HashMap, HashSet};

// Re-export EntityType from core (canonical definition)
pub use brainwires_core::graph::EntityType;

/// A named entity extracted from conversation
#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub entity_type: EntityType,
    pub message_ids: Vec<String>,
    pub first_seen: i64,
    pub last_seen: i64,
    pub mention_count: u32,
}

impl Entity {
    pub fn new(name: String, entity_type: EntityType, message_id: String, timestamp: i64) -> Self {
        Self {
            name,
            entity_type,
            message_ids: vec![message_id],
            first_seen: timestamp,
            last_seen: timestamp,
            mention_count: 1,
        }
    }

    pub fn add_mention(&mut self, message_id: String, timestamp: i64) {
        if !self.message_ids.contains(&message_id) {
            self.message_ids.push(message_id);
        }
        self.last_seen = timestamp.max(self.last_seen);
        self.mention_count += 1;
    }
}

/// Relationship between entities
#[derive(Debug, Clone)]
pub enum Relationship {
    Defines { definer: String, defined: String, context: String },
    References { from: String, to: String },
    Modifies { modifier: String, modified: String, change_type: String },
    DependsOn { dependent: String, dependency: String },
    Contains { container: String, contained: String },
    CoOccurs { entity_a: String, entity_b: String, message_id: String },
}

/// Extraction result from a single message
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    pub entities: Vec<(String, EntityType)>,
    pub relationships: Vec<Relationship>,
}

/// Entity store for tracking entities across a conversation
#[derive(Debug, Default)]
pub struct EntityStore {
    entities: HashMap<String, Entity>,
    relationships: Vec<Relationship>,
}

impl EntityStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_extraction(&mut self, result: ExtractionResult, message_id: &str, timestamp: i64) {
        for (name, entity_type) in result.entities {
            let key = format!("{}:{}", entity_type.as_str(), name);
            if let Some(entity) = self.entities.get_mut(&key) {
                entity.add_mention(message_id.to_string(), timestamp);
            } else {
                self.entities.insert(
                    key,
                    Entity::new(name, entity_type, message_id.to_string(), timestamp),
                );
            }
        }
        self.relationships.extend(result.relationships);
    }

    pub fn get(&self, name: &str, entity_type: &EntityType) -> Option<&Entity> {
        let key = format!("{}:{}", entity_type.as_str(), name);
        self.entities.get(&key)
    }

    pub fn get_by_type(&self, entity_type: &EntityType) -> Vec<&Entity> {
        self.entities.values().filter(|e| &e.entity_type == entity_type).collect()
    }

    pub fn get_top_entities(&self, limit: usize) -> Vec<&Entity> {
        let mut entities: Vec<_> = self.entities.values().collect();
        entities.sort_by(|a, b| b.mention_count.cmp(&a.mention_count));
        entities.into_iter().take(limit).collect()
    }

    pub fn get_related(&self, entity_name: &str) -> Vec<String> {
        let mut related = HashSet::new();
        for rel in &self.relationships {
            match rel {
                Relationship::CoOccurs { entity_a, entity_b, .. } => {
                    if entity_a == entity_name { related.insert(entity_b.clone()); }
                    else if entity_b == entity_name { related.insert(entity_a.clone()); }
                }
                Relationship::Contains { container, contained } => {
                    if container == entity_name { related.insert(contained.clone()); }
                    else if contained == entity_name { related.insert(container.clone()); }
                }
                Relationship::References { from, to } => {
                    if from == entity_name { related.insert(to.clone()); }
                    else if to == entity_name { related.insert(from.clone()); }
                }
                Relationship::DependsOn { dependent, dependency } => {
                    if dependent == entity_name { related.insert(dependency.clone()); }
                    else if dependency == entity_name { related.insert(dependent.clone()); }
                }
                Relationship::Modifies { modifier, modified, .. } => {
                    if modifier == entity_name { related.insert(modified.clone()); }
                    else if modified == entity_name { related.insert(modifier.clone()); }
                }
                Relationship::Defines { definer, defined, .. } => {
                    if definer == entity_name { related.insert(defined.clone()); }
                    else if defined == entity_name { related.insert(definer.clone()); }
                }
            }
        }
        related.into_iter().collect()
    }

    pub fn get_message_ids(&self, entity_name: &str) -> Vec<String> {
        self.entities.values()
            .filter(|e| e.name == entity_name)
            .flat_map(|e| e.message_ids.clone())
            .collect()
    }

    pub fn all_entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    pub fn all_relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    pub fn stats(&self) -> EntityStoreStats {
        let mut by_type = HashMap::new();
        for entity in self.entities.values() {
            *by_type.entry(entity.entity_type.as_str()).or_insert(0) += 1;
        }
        EntityStoreStats {
            total_entities: self.entities.len(),
            total_relationships: self.relationships.len(),
            entities_by_type: by_type,
        }
    }
}

impl brainwires_core::graph::EntityStoreT for EntityStore {
    fn entity_names_by_type(&self, entity_type: &EntityType) -> Vec<String> {
        self.get_by_type(entity_type)
            .iter()
            .map(|e| e.name.clone())
            .collect()
    }

    fn top_entity_info(&self, limit: usize) -> Vec<(String, EntityType)> {
        self.get_top_entities(limit)
            .iter()
            .map(|e| (e.name.clone(), e.entity_type.clone()))
            .collect()
    }
}

#[derive(Debug)]
pub struct EntityStoreStats {
    pub total_entities: usize,
    pub total_relationships: usize,
    pub entities_by_type: HashMap<&'static str, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_type_as_str() {
        assert_eq!(EntityType::File.as_str(), "file");
        assert_eq!(EntityType::Function.as_str(), "function");
    }

    #[test]
    fn test_entity_lifecycle() {
        let mut entity = Entity::new("main.rs".into(), EntityType::File, "msg-1".into(), 100);
        assert_eq!(entity.mention_count, 1);
        entity.add_mention("msg-2".into(), 200);
        assert_eq!(entity.mention_count, 2);
        assert_eq!(entity.last_seen, 200);
    }

    #[test]
    fn test_entity_store() {
        let mut store = EntityStore::new();
        let result = ExtractionResult {
            entities: vec![
                ("main.rs".into(), EntityType::File),
                ("process".into(), EntityType::Function),
            ],
            relationships: vec![],
        };
        store.add_extraction(result, "msg-1", 100);
        assert_eq!(store.stats().total_entities, 2);
    }
}
