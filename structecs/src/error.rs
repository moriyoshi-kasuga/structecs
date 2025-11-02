use std::fmt;

use crate::EntityId;

/// Errors that can occur when interacting with the World.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldError {
    /// The specified entity was not found in the world.
    EntityNotFound(EntityId),

    /// The requested component type was not found on the entity.
    ComponentNotFound {
        entity_id: EntityId,
        component_name: &'static str,
    },

    /// The requested additional component was not found on the entity.
    AdditionalNotFound {
        entity_id: EntityId,
        component_name: &'static str,
    },

    /// The archetype for the entity was not found (internal consistency error).
    ArchetypeNotFound(EntityId),
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldError::EntityNotFound(id) => {
                write!(f, "Entity {} not found in world", id)
            }
            WorldError::ComponentNotFound {
                entity_id,
                component_name,
            } => {
                write!(
                    f,
                    "Component '{}' not found on entity {}",
                    component_name, entity_id
                )
            }
            WorldError::AdditionalNotFound {
                entity_id,
                component_name,
            } => {
                write!(
                    f,
                    "Additional component '{}' not found on entity {}",
                    component_name, entity_id
                )
            }
            WorldError::ArchetypeNotFound(id) => {
                write!(
                    f,
                    "Archetype not found for entity {} (internal error)",
                    id
                )
            }
        }
    }
}

impl std::error::Error for WorldError {}
