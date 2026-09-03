#![cfg(feature = "tokio")]

use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};
use std::sync::{Arc, Mutex};

#[derive(MediCommand)]
#[medi_command(error_type = medi_rs::Error)]
struct CreateUser {
    name: String,
}
struct User {
    name: String,
}
trait UserRepository: Send + Sync {
    fn save(&self, user: User) -> Result<()>;
}
#[derive(Clone)]
struct InMemoryUserRepository(Arc<Mutex<Vec<User>>>);
impl InMemoryUserRepository {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
}
impl UserRepository for InMemoryUserRepository {
    fn save(&self, user: User) -> Result<()> {
        self.0.lock().unwrap().push(user);
        Ok(())
    }
}

#[derive(Clone)]
struct AppStateDyn {
    user_repository: Arc<dyn UserRepository>,
}
#[derive(Clone)]
struct AppStateGeneric<T: UserRepository> {
    user_repository: Arc<T>,
}

#[medi_handler]
async fn create_user_dyn(state: AppStateDyn, req: CreateUser) -> Result<()> {
    state.user_repository.save(User { name: req.name })
}
#[medi_handler]
async fn create_user_generic(state: AppStateGeneric<InMemoryUserRepository>, req: CreateUser) -> Result<()> {
    state.user_repository.save(User { name: req.name })
}

medi_module! { manifest dyn_manifest; resources { AppStateDyn; } commands { CreateUser => create_user_dyn; } }
medi_module! { manifest generic_manifest; resources { AppStateGeneric<InMemoryUserRepository>; } commands { CreateUser => create_user_generic; } }

mediator! { pub struct DynMediator { event_queue_capacity: 1; event_workers: 1; modules: [dyn_manifest]; } }
mediator! { pub struct GenericMediator { event_queue_capacity: 1; event_workers: 1; modules: [generic_manifest]; } }

#[tokio::test]
async fn send_should_work_with_dependencyinjection() {
    let repo = Arc::new(InMemoryUserRepository::new());
    let mediator = DynMediator::new((AppStateDyn {
        user_repository: repo.clone(),
    },));
    mediator.send(CreateUser { name: "John".into() }).await.unwrap();
    assert_eq!(repo.0.lock().unwrap()[0].name, "John");
}

#[tokio::test]
async fn send_should_work_with_generic_dependencyinjection() {
    let repo = Arc::new(InMemoryUserRepository::new());
    let mediator = GenericMediator::new((AppStateGeneric {
        user_repository: repo.clone(),
    },));
    mediator.send(CreateUser { name: "John".into() }).await.unwrap();
    assert_eq!(repo.0.lock().unwrap()[0].name, "John");
}
