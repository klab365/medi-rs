use core::pin::pin;
use medi_rs::stream::{StreamExt, TryStreamExt};
use medi_rs::{MediStreamRequest, StreamSender, medi_module, medi_stream_handler, mediator};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
enum SearchError {
    #[error("query must not be empty")]
    EmptyQuery,
}

/// Resource injected into the stream handler.
#[derive(Clone)]
struct UserDirectory {
    users: &'static [&'static str],
}

#[derive(MediStreamRequest)]
#[medi_stream(item_type = String, error_type = SearchError, capacity = 2)]
struct SearchUsers {
    query: &'static str,
}

#[medi_stream_handler]
async fn search_users(
    directory: UserDirectory,
    sender: StreamSender<'_, SearchUsers>,
    request: SearchUsers,
) -> Result<(), SearchError> {
    if request.query.is_empty() {
        return Err(SearchError::EmptyQuery);
    }
    for user in directory.users.iter().filter(|user| user.contains(request.query)) {
        // Simulate a paged lookup; each result is delivered as soon as it is found.
        tokio::time::sleep(Duration::from_millis(50)).await;
        sender.send((*user).to_owned()).await;
    }
    Ok(())
}

medi_module! {
    manifest users_manifest;
    resources { user_directory: UserDirectory; }
    streams { SearchUsers => search_users; }
}

mediator! {
    pub struct UserMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [users_manifest];
    }
}

#[tokio::main]
async fn main() -> Result<(), SearchError> {
    let mediator = UserMediator::builder()
        .user_directory(UserDirectory {
            users: &["Ada Lovelace", "Alan Turing", "Ada Yonath", "Grace Hopper"],
        })
        .build();

    // Recommended: `try_for_each` pins internally and returns handler errors via `?`.
    mediator
        .stream(SearchUsers { query: "Ada" })
        .try_for_each(|user| async move {
            println!("found: {user}");
            Ok(())
        })
        .await?;

    // `try_collect` gathers all items or returns the handler error.
    let all: Vec<String> = mediator.stream(SearchUsers { query: "a" }).try_collect().await?;
    println!("all matches: {all:?}");
    match mediator.stream(SearchUsers { query: "" }).try_collect::<Vec<_>>().await {
        Ok(_) => unreachable!("an empty query is rejected"),
        Err(error) => println!("search failed: {error}"),
    }

    // A manual `next()` loop needs `pin!`; use it for control flow such as `break`.
    // Dropping the stream after `break` cancels the handler.
    let mut users = pin!(mediator.stream(SearchUsers { query: "a" }));
    while let Some(user) = users.next().await {
        let user = user?;
        println!("checking: {user}");
        if user.starts_with("Alan") {
            println!("found Alan, stopping early");
            break;
        }
    }
    Ok(())
}
