use std::convert::TryFrom;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

#[derive(Clone)]
pub struct TicketStore {
    tickets: BTreeMap<TicketId, Arc<RwLock<Ticket>>>,
    counter: u64,
}

impl TicketStore {
    pub fn new() -> Self {
        Self {
            tickets: BTreeMap::new(),
            counter: 0,
        }
    }

    pub fn add_ticket(&mut self, ticket: TicketDraft) -> TicketId {
        let id = TicketId(self.counter);
        self.counter += 1;
        let ticket = Ticket {
            id,
            title: ticket.title,
            description: ticket.description,
            status: Status::ToDo,
        };
        let ticket = Arc::new(RwLock::new(ticket));
        self.tickets.insert(id, ticket);
        id
    }

    pub fn get(&self, id: TicketId) -> Option<Arc<RwLock<Ticket>>> {
        self.tickets.get(&id).cloned()
    }
    
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ticket {
    pub id: TicketId,
    pub title: TicketTitle,
    pub description: TicketDescription,
    pub status: Status,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketDraft {
    pub title: TicketTitle,
    pub description: TicketDescription,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketPatch {
    pub id: TicketId,
    pub title: Option<TicketTitle>,
    pub description: Option<TicketDescription>,
    pub status: Option<Status>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TicketId(u64);

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    ToDo,
    InProgress,
    Done,
}

/// Description

#[derive(Debug, PartialEq, Clone, Eq, Serialize, Deserialize)]
pub struct TicketDescription(String);

#[derive(Debug, thiserror::Error)]
pub enum TicketDescriptionError {
    #[error("The description cannot be empty")]
    Empty,
    #[error("The description cannot be longer than 500 bytes")]
    TooLong,
}

impl TryFrom<String> for TicketDescription {
    type Error = TicketDescriptionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_description(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&str> for TicketDescription {
    type Error = TicketDescriptionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        validate_description(value)?;
        Ok(Self(value.to_string()))
    }
}

fn validate_description(description: &str) -> Result<(), TicketDescriptionError> {
    if description.is_empty() {
        Err(TicketDescriptionError::Empty)
    } else if description.len() > 500 {
        Err(TicketDescriptionError::TooLong)
    } else {
        Ok(())
    }
}

/// Title

#[derive(Debug, PartialEq, Clone, Eq, Serialize, Deserialize)]
pub struct TicketTitle(String);

#[derive(Debug, thiserror::Error)]
pub enum TicketTitleError {
    #[error("The title cannot be empty")]
    Empty,
    #[error("The title cannot be longer than 50 bytes")]
    TooLong,
}

impl TryFrom<String> for TicketTitle {
    type Error = TicketTitleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_title(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&str> for TicketTitle {
    type Error = TicketTitleError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        validate_title(value)?;
        Ok(Self(value.to_string()))
    }
}

fn validate_title(title: &str) -> Result<(), TicketTitleError> {
    if title.is_empty() {
        Err(TicketTitleError::Empty)
    } else if title.len() > 50 {
        Err(TicketTitleError::TooLong)
    } else {
        Ok(())
    }
}


pub fn ticket_title() -> TicketTitle {
    valid_title().try_into().unwrap()
}

pub fn ticket_description() -> TicketDescription {
    valid_description().try_into().unwrap()
}

pub fn overly_long_description() -> String {
    "At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum deleniti atque corrupti quos dolores et quas molestias excepturi sint occaecati cupiditate non provident, similique sunt in culpa qui officia deserunt mollitia animi, id est laborum et dolorum fuga. Et harum quidem rerum facilis est et expedita distinctio. Nam libero tempore, cum soluta nobis est eligendi optio cumque nihil impedit quo minus id quod maxime placeat facere possimus, omnis voluptas assumenda est, omnis dolor repellendus. Temporibus autem quibusdam et aut officiis debitis aut rerum necessitatibus saepe eveniet ut et voluptates repudiandae sint et molestiae non recusandae. Itaque earum rerum hic tenetur a sapiente delectus, ut aut reiciendis voluptatibus maiores alias consequatur aut perferendis doloribus asperiores repellat.".into()
}

pub fn overly_long_title() -> String {
    "A title that's definitely longer than what should be allowed in a development ticket".into()
}

pub fn valid_title() -> String {
    "A title".into()
}

pub fn valid_description() -> String {
    "A description".into()
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;
    use crate::Command;

    #[test]
    fn test_try_desc_from_string() {
        let input = valid_description();
        let description = TicketDescription::try_from(input.clone()).unwrap();
        assert_eq!(description.0, input);
    }

    #[test]
    fn test_try_desc_from_empty_string() {
        let err = TicketDescription::try_from("".to_string()).unwrap_err();
        assert_eq!(err.to_string(), "The description cannot be empty");
    }

    #[test]
    fn test_try_desc_from_long_string() {
        let err = TicketDescription::try_from(overly_long_description()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "The description cannot be longer than 500 bytes"
        );
    }

    #[test]
    fn test_try_desc_from_str() {
        let description = TicketDescription::try_from("A description").unwrap();
        assert_eq!(description.0, "A description");
    }

    #[test]
    fn test_try_title_from_string() {
        let input = valid_title();
        let title = TicketTitle::try_from(input.clone()).unwrap();
        assert_eq!(title.0, input);
    }

    #[test]
    fn test_try_title_from_empty_string() {
        let err = TicketTitle::try_from("".to_string()).unwrap_err();
        assert_eq!(err.to_string(), "The title cannot be empty");
    }

    #[test]
    fn test_try_title_from_long_string() {
        let err = TicketTitle::try_from(overly_long_title()).unwrap_err();
        assert_eq!(err.to_string(), "The title cannot be longer than 50 bytes");
    }

    #[test]
    fn test_try_title_from_str() {
        let title = TicketTitle::try_from("A title").unwrap();
        assert_eq!(title.0, "A title");
    }

    #[test]
    fn serialize_ticket_draft() {
        let draft = TicketDraft {
            title: ticket_title(),
            description: ticket_description()
        };

        let bytes = serde_json::to_vec(&draft).unwrap();
        let result: TicketDraft = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(result, draft)
    }
    #[test]
    fn serialize_ticket() {
        let ticket = Ticket {
            title: ticket_title(),
            description: ticket_description(),
            id: TicketId(0),
            status: Status::InProgress
        };

        let bytes = serde_json::to_vec(&ticket).unwrap();
        let result: Ticket = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(result, ticket)
    }
    #[test]
    fn serialize_command() {
        let command = Command::Insert {
            draft: TicketDraft {
                title: ticket_title(),
                description: ticket_description(),
            }
        };

        let bytes = serde_json::to_vec(&command).unwrap();
        let result: Command = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(result, command)
    }
}