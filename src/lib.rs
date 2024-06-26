// This is our last exercise. Let's go down a more unstructured path!
// Try writing an **asynchronous REST API** to expose the functionality
// of the ticket management system we built throughout the course.
// It should expose endpoints to:
//  - Create a ticket
//  - Retrieve ticket details
//  - Patch a ticket
//
// Use Rust's package registry, crates.io, to find the dependencies you need
// (if any) to build this system.

use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
use std::str;
use std::sync::Arc;
use tokio::sync::RwLock;
mod data;
use data::*;
use serde::{Serialize, Deserialize};


#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Command {
    Insert {
        draft: TicketDraft,
    },
    Get {
        id: TicketId,
    },
    Update {
        patch: TicketPatch,
    },
}

pub async fn ticket_server(first: TcpListener) -> Result<(), anyhow::Error> {
    let handle1 = tokio::spawn(ticket_handler(first));
    handle1.await.unwrap()
}

async fn ticket_handler(listener: TcpListener) -> Result<(), anyhow::Error> {
    let store = TicketStore::new();
    let store = Arc::new(RwLock::new(store));
    loop {
        let (mut socket, _) = listener.accept().await?;
        let store_client = store.clone();
        let response_handle = tokio::spawn(async move {
            let (mut reader, mut writer) = socket.split();
            let mut buf: Vec<u8> = Vec::new();
            reader.read_to_end(&mut buf).await.unwrap();

            let request: Command = serde_json::from_slice(&buf).unwrap();

            let response = match request {
                Command::Insert {draft}=> {
                    let id = store_client.write().await.add_ticket(draft);
                    let response = serde_json::to_vec(&id);
                    response
                },
                Command::Get  {id}=> {
                    let store_reader = store_client.read().await;
                    let ticket = store_reader.get(id).unwrap();
                    let ticket = ticket.read().await;
                    let response = serde_json::to_vec(&ticket.clone());
                    response
                },
                Command::Update{patch} => {
                    let store_reader = store_client.read().await;
                    if let Some(ticket_locked) = store_reader.get(patch.id) {
                        let mut ticket = ticket_locked.write().await;
                        if let Some(title) = patch.title {
                            ticket.title = title;
                        }
                        if let Some(description) = patch.description {
                            ticket.description = description;
                        }
                        if let Some(status) = patch.status {
                            ticket.status = status;
                        }
                    }
                    let response = serde_json::to_vec(&patch.id);
                    response
                }

            }.unwrap();

            //println!("Responded!");
            writer.write_all(&response).await.unwrap();
        });

        response_handle.await.unwrap();
    }
}

#[derive(Clone, Copy, Debug,)]
pub struct TicketClient {
    addr: SocketAddr,
}

impl TicketClient {
    pub fn new(addr: SocketAddr) -> Self {
        TicketClient {
            addr
        }
    }
    pub async fn insert(self, draft: TicketDraft) -> TicketId {
        let req = Command::Insert {
            draft
        };
        let mut socket = tokio::net::TcpStream::connect(self.addr).await.unwrap();
        let (mut reader, mut writer) = socket.split();
    
        let request_formatted = serde_json::to_vec(&req).unwrap();
        writer.write_all(&request_formatted).await.unwrap();
        writer.shutdown().await.unwrap();
    
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await.unwrap();
        let response_formatted: TicketId = serde_json::from_slice(&buf).unwrap();
        response_formatted
    }
    
    pub async fn get(self, id: TicketId) -> Ticket {
        let req = Command::Get {
            id
        };
        let mut socket = tokio::net::TcpStream::connect(self.addr).await.unwrap();
        let (mut reader, mut writer) = socket.split();
    
        let request_formatted = serde_json::to_vec(&req).unwrap();
        writer.write_all(&request_formatted).await.unwrap();
        writer.shutdown().await.unwrap();
        //println!("Requested with {:#?}", &id);
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await.unwrap();
        let response_formatted: Ticket = serde_json::from_slice(&buf).unwrap();
        response_formatted
    }
    
    pub async fn update(self, patch: TicketPatch) -> TicketId {
        let req = Command::Update {
            patch
        };
        let mut socket = tokio::net::TcpStream::connect(self.addr).await.unwrap();
        let (mut reader, mut writer) = socket.split();
    
        let request_formatted = serde_json::to_vec(&req).unwrap();
        writer.write_all(&request_formatted).await.unwrap();
        writer.shutdown().await.unwrap();
        //println!("Requested with {:#?}", &id);
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await.unwrap();
        let response_formatted: TicketId = serde_json::from_slice(&buf).unwrap();
        response_formatted
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::panic;
    use tokio::task::JoinSet;

    async fn bind_random() -> (TcpListener, SocketAddr) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        (listener, addr)
    }

    #[tokio::test]
    async fn test_insert_get_tickets() {
        let (listener, addr) = bind_random().await;
        tokio::spawn(ticket_server(listener));
        // server is running, begin sending tasks
        let client = TicketClient::new(addr);

        let draft1 =  TicketDraft {
            title: ticket_title(),
            description: ticket_description(),
        };

        let draft2 =  TicketDraft {
            title: ticket_title(),
            description: ticket_description(),
        };
        // issue insert requests
        let mut insert_join_set = JoinSet::new();
        insert_join_set.spawn(client.insert(draft1));
        insert_join_set.spawn(client.insert(draft2));

        let mut ticket_ids = Vec::new();
        while let Some(outcome) = insert_join_set.join_next().await {
            match outcome {
                Err(e) => {
                    if let Ok(reason) = e.try_into_panic() {
                        panic::resume_unwind(reason);
                    }
                },
                Ok(val) => ticket_ids.push(val), 
            }
        }
        // println!("Returned with {:#?}", &ticket_ids);

        let ticket_id1 = ticket_ids[0];
        let ticket_id2 = ticket_ids[1];
        // issue get requests
        let mut get_join_set = JoinSet::new();
        get_join_set.spawn(client.get(ticket_id1));
        get_join_set.spawn(client.get(ticket_id2));

        let mut tickets: Vec<Ticket> = Vec::new();
        while let Some(outcome) = get_join_set.join_next().await {
            match outcome {
                Err(e) => {
                    if let Ok(reason) = e.try_into_panic() {
                        panic::resume_unwind(reason);
                    }
                },
                Ok(val) => tickets.push(val),
            }
        }

        // println!("Returned with {:#?}", &tickets);
        assert_eq!(tickets[0].id, ticket_ids[0]);
        assert_eq!(tickets[0].title, ticket_title());
        assert_eq!(tickets[0].description, ticket_description());
        assert_eq!(tickets[1].id, ticket_ids[1]);

    }

    #[tokio::test]
    async fn test_update_tickets() {
        let (listener, addr) = bind_random().await;
        tokio::spawn(ticket_server(listener));

        let client = TicketClient::new(addr);

        let draft1 =  TicketDraft {
            title: ticket_title(),
            description: ticket_description(),
        };

        // issue insert requests
        let mut insert_join_set = JoinSet::new();
        insert_join_set.spawn(client.insert(draft1));

        let mut ticket_ids = Vec::new();
        while let Some(outcome) = insert_join_set.join_next().await {
            match outcome {
                Err(e) => {
                    if let Ok(reason) = e.try_into_panic() {
                        panic::resume_unwind(reason);
                    }
                },
                Ok(val) => ticket_ids.push(val), 
            }
        }
        // println!("Returned with {:#?}", &ticket_ids);

        let ticket_id1 = ticket_ids[0];

        let ticket_patch1 = TicketPatch {
            id: ticket_id1.clone(),
            title: Some(TicketTitle::try_from("Modified").unwrap()),
            description: Some(TicketDescription::try_from("Modified as well").unwrap()),
            status: Some(Status::InProgress)
        };

        // issue update requests
        let mut update_join_set = JoinSet::new();
        update_join_set.spawn(client.update(ticket_patch1));

        let mut ticket_ids2: Vec<TicketId> = Vec::new();
        while let Some(outcome) = update_join_set.join_next().await {
            match outcome {
                Err(e) => {
                    if let Ok(reason) = e.try_into_panic() {
                        panic::resume_unwind(reason);
                    }
                },
                Ok(val) => ticket_ids2.push(val),
            }
        }
        // println!("Returned with {:#?}", &ticket_ids2);
        assert_eq!(&ticket_ids[0], &ticket_ids2[0]);

        // issue get requests
        let mut get_join_set = JoinSet::new();
        get_join_set.spawn(client.get(ticket_id1));

        let mut patched_tickets: Vec<Ticket> = Vec::new();
        while let Some(outcome) = get_join_set.join_next().await {
            match outcome {
                Err(e) => {
                    if let Ok(reason) = e.try_into_panic() {
                        panic::resume_unwind(reason);
                    }
                },
                Ok(val) => patched_tickets.push(val),
            }
        }

        // println!("Returned with {:#?}", &patched_tickets);
        assert_eq!(patched_tickets[0].id, ticket_ids[0]);
        assert_eq!(patched_tickets[0].title, TicketTitle::try_from("Modified").unwrap());
        assert_eq!(patched_tickets[0].description, TicketDescription::try_from("Modified as well").unwrap());

    }

}