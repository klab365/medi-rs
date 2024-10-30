mod bus_builder;

// -- flatten
pub use bus_builder::BusBuilder;
use tokio::sync::mpsc::{self, Receiver, Sender};

// -- use dependencies
use crate::error::{Error, HandlerResult};
use crate::event::{EventWrapper, EventWrapperTrait};
use crate::handler_wrapper::HandlerWrapperTrait;
use crate::{FromResources, IntoCommand, IntoEvent, Resources, SharedHandler};
use std::any::TypeId;
use std::sync::Arc;

type EventQueueItem = Box<dyn EventWrapperTrait + Send + Sync>;

#[derive(Clone)]
pub struct Bus {
    req_handlers: SharedHandler<Arc<dyn HandlerWrapperTrait>>,
    evt_handlers: SharedHandler<Vec<Arc<dyn HandlerWrapperTrait>>>,
    resources: Resources,
    pending_events: Sender<EventQueueItem>,
}

impl FromResources for Bus {}

impl Bus {
    pub fn builder() -> BusBuilder {
        BusBuilder::default()
    }
}

impl Bus {
    pub(crate) fn new(
        resources: Resources,
        req_handlers: SharedHandler<Arc<dyn HandlerWrapperTrait>>,
        evt_handlers: SharedHandler<Vec<Arc<dyn HandlerWrapperTrait>>>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(1024);
        let mut bus = Bus {
            req_handlers,
            evt_handlers,
            resources,
            pending_events: tx,
        };

        // add bus to resources
        bus.resources.insert(bus.clone());

        // start processing events
        bus.start_processing_events(rx);

        bus
    }

    pub async fn send<Req, Res>(&self, req: Req) -> HandlerResult<Res>
    where
        Req: IntoCommand<Res> + Send + Sync + 'static,
        Res: Send + Sync + 'static,
    {
        let type_id = TypeId::of::<Req>();

        let handler = self.req_handlers.get(&type_id);
        let Some(handler) = handler else {
            return Err(Error::HandlerNotFound);
        };

        let req = Box::new(req);
        let res = handler.handle(self.resources.clone(), req).await?;

        let Ok(res) = res.downcast::<Res>() else {
            let type_name = std::any::type_name::<Res>();
            return Err(Error::CastError(type_name.to_string()));
        };

        Ok(*res)
    }

    pub async fn publish<Evt>(&self, evt: Evt) -> HandlerResult<()>
    where
        Evt: IntoEvent + Clone + Send + Sync + 'static,
    {
        let event_wrapper = EventWrapper::new(evt);
        let event_item = event_wrapper.into_dyn();
        self.pending_events
            .send(event_item)
            .await
            .map_err(|_| Error::EventProcessingError)?;

        Ok(())
    }

    fn start_processing_events(&self, rx: Receiver<EventQueueItem>) {
        let bus = self.clone();
        tokio::spawn(async move {
            test_loop(Arc::new(bus), rx).await;
        });
    }
}

async fn test_loop(bus: Arc<Bus>, mut rx: Receiver<EventQueueItem>) {
    while let Some(event_item) = rx.recv().await {
        let event_item_type = event_item.get_type_id();
        let Some(handlers) = bus.evt_handlers.get(&event_item_type) else {
            eprintln!("Handler not found for event: {:?}", event_item.get_type_id());
            continue;
        };

        for handler in handlers {
            let evt = event_item.get_any();
            let res = handler.handle(bus.resources.clone(), evt).await;
            if let Err(e) = res {
                eprintln!("Error: {:?}", e);
            }
        }
    }
}
