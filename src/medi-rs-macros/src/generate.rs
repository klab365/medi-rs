//! Code generation for composed mediators.

use crate::handler::decorate_handler_call;
use crate::manifest::{ModuleManifest, handler_invoker_path};
use crate::task::task_invoker_path;
use quote::{format_ident, quote};
use syn::{Expr, Ident, Path, Type};

pub(crate) fn collect_event_routes(modules: &[ModuleManifest]) -> Vec<(Type, Vec<syn::Path>)> {
    let mut routes: Vec<(Type, Vec<syn::Path>)> = Vec::new();
    for event in modules.iter().flat_map(|module| &module.events) {
        let event_type = &event.event;
        let key = quote!(#event_type).to_string();
        if let Some((_, handlers)) = routes
            .iter_mut()
            .find(|(registered, _)| quote!(#registered).to_string() == key)
        {
            handlers.extend(event.handlers.iter().cloned());
        } else {
            routes.push((event.event.clone(), event.handlers.clone()));
        }
    }
    routes
}

/// Generated fragments for event-enabled mediators.
pub(crate) struct EventSupport {
    pub(crate) job: proc_macro2::TokenStream,
    pub(crate) field: proc_macro2::TokenStream,
    pub(crate) publish_routes: proc_macro2::TokenStream,
    pub(crate) publish_method: proc_macro2::TokenStream,
    pub(crate) worker: proc_macro2::TokenStream,
}

pub(crate) struct EventDispatchConfig<'a> {
    pub(crate) decorators: &'a [Path],
    pub(crate) event_failure_reporter: Option<&'a Type>,
}

pub(crate) fn collect_tasks(modules: &[ModuleManifest]) -> Vec<syn::Path> {
    modules.iter().flat_map(|module| module.tasks.iter().cloned()).collect()
}

pub(crate) fn collect_resource_types(modules: &[ModuleManifest]) -> Vec<Type> {
    modules
        .iter()
        .flat_map(|module| module.resources.iter().cloned())
        .collect()
}

pub(crate) fn generate_constructor(
    resource_types: &[Type],
    resource_names: &[Ident],
    resource_values: &proc_macro2::TokenStream,
    has_events: bool,
    capacity: &Expr,
    task_count: usize,
    event_failure_reporter: Option<&Type>,
) -> proc_macro2::TokenStream {
    let task_shutdown_fields = (0..task_count).map(|index| {
        let field = format_ident!("task_shutdown_{index}");
        quote! { #field: ::medi_rs::ShutdownSignal::new(), }
    });
    let configuration_check = quote! {
        assert!(#capacity > 0, "event_queue_capacity must be greater than zero");
    };
    let reporter_value = event_failure_reporter.map(|reporter| quote! { event_failure_reporter: #reporter, });
    match (resource_types.is_empty(), has_events) {
        (true, true) => quote! {
            /// Construct a mediator with no typed resources.
            pub fn new() -> Self {
                #configuration_check
                Self {
                    resources: (),
                    event_queue: ::medi_rs::EventQueue::new(Some(#capacity)),
                    #reporter_value
                    #(#task_shutdown_fields)*
                    lifecycle: ::medi_rs::Lifecycle::new(),
                }
            }
        },
        (true, false) => quote! {
            /// Construct a mediator with no typed resources.
            pub fn new() -> Self {
                Self { resources: (), #(#task_shutdown_fields)* lifecycle: ::medi_rs::Lifecycle::new() }
            }
        },
        (false, true) => quote! {
            /// Construct a mediator from its declared resource values.
            pub fn new(#(#resource_names: #resource_types),*) -> Self {
                #configuration_check
                Self {
                    resources: #resource_values,
                    event_queue: ::medi_rs::EventQueue::new(Some(#capacity)),
                    #reporter_value
                    #(#task_shutdown_fields)*
                    lifecycle: ::medi_rs::Lifecycle::new(),
                }
            }
        },
        (false, false) => quote! {
            /// Construct a mediator from its declared resource values.
            pub fn new(#(#resource_names: #resource_types),*) -> Self {
                Self { resources: #resource_values, #(#task_shutdown_fields)* lifecycle: ::medi_rs::Lifecycle::new() }
            }
        },
    }
}

pub(crate) fn generate_task_workers(tasks: &[syn::Path], name: &Ident) -> Vec<proc_macro2::TokenStream> {
    tasks
        .iter()
        .enumerate()
        .map(|(index, task)| {
            let worker = format_ident!("medi_rs_task_{index}");
            let task_shutdown = format_ident!("task_shutdown_{index}");
            let invoker = task_invoker_path(task);
            if cfg!(feature = "embassy") {
                quote! {
                    #[::medi_rs::embassy_executor::task]
                    async fn #worker(mediator: &'static #name) {
                        #invoker(mediator, &mediator.resources, &mediator.#task_shutdown).await;
                        mediator.lifecycle.finish();
                    }
                }
            } else {
                quote! {
                    async fn #worker(mediator: &'static #name) {
                        #invoker(mediator, &mediator.resources, &mediator.#task_shutdown).await;
                        mediator.lifecycle.finish();
                    }
                }
            }
        })
        .collect()
}

pub(crate) fn generate_task_spawns(tasks: &[syn::Path]) -> Vec<proc_macro2::TokenStream> {
    tasks
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let worker = format_ident!("medi_rs_task_{index}");
            if cfg!(feature = "embassy") {
                quote! {
                    self.lifecycle.begin();
                    if let Ok(token) = #worker(self) {
                        spawner.spawn(token);
                    } else {
                        self.lifecycle.finish();
                    }
                }
            } else {
                quote! {
                    self.lifecycle.begin();
                    ::medi_rs::adapters::selected::spawn(#worker(self));
                }
            }
        })
        .collect()
}

pub(crate) fn generate_command_routes(
    modules: &[ModuleManifest],
    name: &Ident,
    decorators: &[Path],
) -> Vec<proc_macro2::TokenStream> {
    modules.iter().flat_map(|module| module.commands.iter().map(|command| {
        let request = &command.request;
        let invoker = handler_invoker_path(&command.handler);
        let invocation = if decorators.is_empty() {
            quote! { #invoker(mediator, &mediator.resources, self).await }
        } else {
            decorate_handler_call(decorators, quote! { self }, &quote! { #invoker(mediator, &mediator.resources, message).await })
        };
        quote! {
            impl ::medi_rs::StaticSendCommand<#name> for #request {
                type Response = <#request as ::medi_rs::Command>::Response;
                type Error = <#request as ::medi_rs::StaticCommand>::Error;
                fn send(self, mediator: &#name) -> impl core::future::Future<Output = core::result::Result<Self::Response, Self::Error>> + Send {
                    async move { #invocation }
                }
            }
        }
    })).collect()
}

fn generate_event_dispatch_arms(
    routes: &[(Type, Vec<syn::Path>)],
    job: &Ident,
    decorators: &[Path],
    has_failure_reporter: bool,
) -> Vec<proc_macro2::TokenStream> {
    routes
        .iter()
        .enumerate()
        .map(|(index, (event_type, handlers))| {
            let variant = format_ident!("Event{index}");
            let calls = handlers.iter().map(|handler| {
                let invoker = handler_invoker_path(handler);
                let invocation = if decorators.is_empty() {
                    quote! { #invoker(mediator, &mediator.resources, event.clone()).await }
                } else {
                    decorate_handler_call(
                        decorators,
                        quote! { event.clone() },
                        &quote! { #invoker(mediator, &mediator.resources, message).await },
                    )
                };
                if has_failure_reporter {
                    quote! {
                        if (#invocation).is_err() {
                            ::medi_rs::EventFailureReporter::report(
                                &mediator.event_failure_reporter,
                                ::medi_rs::EventHandlerFailure::new(stringify!(#event_type), stringify!(#handler)),
                            ).await;
                        }
                    }
                } else {
                    quote! { let _ = #invocation; }
                }
            });
            quote! { #job::#variant(event) => { #(#calls)* } }
        })
        .collect()
}

fn generate_event_start(
    resource_tuple: &proc_macro2::TokenStream,
    worker: &Ident,
    task_spawns: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    if cfg!(feature = "embassy") {
        quote! {
            /// Start generated Embassy event workers and registered runtime tasks.
            ///
            /// Returns [`::medi_rs::StartError::AlreadyStarted`] without
            /// spawning work when this mediator was already started.
            pub fn start(&'static self, spawner: ::medi_rs::embassy_executor::Spawner) -> core::result::Result<(), ::medi_rs::StartError> where #resource_tuple: Sync {
                assert!(Self::EVENT_WORKERS > 0, "event_workers must be greater than zero");
                self.lifecycle.start()?;
                let event_workers = Self::EVENT_WORKERS;
                for _ in 0..event_workers {
                    self.lifecycle.begin();
                    if let Ok(token) = #worker(self) {
                        spawner.spawn(token);
                    } else {
                        self.lifecycle.finish();
                    }
                }
                #(#task_spawns)*
                Ok(())
            }
        }
    } else {
        quote! {
            /// Start generated event workers and registered runtime tasks.
            ///
            /// Returns [`::medi_rs::StartError::AlreadyStarted`] without
            /// spawning work when this mediator was already started.
            pub fn start(&'static self) -> core::result::Result<(), ::medi_rs::StartError> where #resource_tuple: Sync {
                assert!(Self::EVENT_WORKERS > 0, "event_workers must be greater than zero");
                self.lifecycle.start()?;
                let event_workers = Self::EVENT_WORKERS;
                for _ in 0..event_workers {
                    self.lifecycle.begin();
                    ::medi_rs::adapters::selected::spawn(#worker(self));
                }
                #(#task_spawns)*
                Ok(())
            }
        }
    }
}

pub(crate) fn generate_event_support(
    routes: &[(Type, Vec<syn::Path>)],
    name: &Ident,
    job: &Ident,
    capacity: &Expr,
    resource_tuple: &proc_macro2::TokenStream,
    config: EventDispatchConfig<'_>,
    task_spawns: &[proc_macro2::TokenStream],
) -> EventSupport {
    let variants: Vec<_> = routes
        .iter()
        .enumerate()
        .map(|(index, (event, _))| {
            let variant = format_ident!("Event{index}");
            quote! { #variant(#event) }
        })
        .collect();
    let publish_routes: Vec<_> = routes
        .iter()
        .enumerate()
        .map(|(index, (event, _))| {
            let variant = format_ident!("Event{index}");
            quote! { impl ::medi_rs::StaticPublish<#name> for #event where #event: Clone + Send + 'static {
                fn publish(self, mediator: &#name) -> impl core::future::Future<Output = ::medi_rs::Result<()>> + Send {
                    ::medi_rs::EventQueue::publish(&mediator.event_queue, #job::#variant(self))
                }
            }
            impl ::medi_rs::StaticTryPublish<#name> for #event where #event: Clone + Send + 'static {
                fn try_publish(self, mediator: &#name) -> core::result::Result<(), ::medi_rs::TryPublishError<Self>> {
                    match ::medi_rs::EventQueue::try_publish(&mediator.event_queue, #job::#variant(self.clone())) {
                        Ok(()) => Ok(()),
                        Err(::medi_rs::TryPublishError::Full(_)) => Err(::medi_rs::TryPublishError::Full(self)),
                        Err(::medi_rs::TryPublishError::Closed(_)) => Err(::medi_rs::TryPublishError::Closed(self)),
                    }
                }
            } }
        })
        .collect();
    let dispatch_arms =
        generate_event_dispatch_arms(routes, job, config.decorators, config.event_failure_reporter.is_some());
    let worker = format_ident!("medi_rs_event_worker");
    let worker_loop = quote! {
        loop {
            match ::medi_rs::EventQueue::recv(&mediator.event_queue).await {
                Ok(#job::Shutdown) | Err(_) => break,
                Ok(event) => match event {
                    #(#dispatch_arms)*
                    #job::Shutdown => break,
                },
            }
        }
        mediator.lifecycle.finish();
    };
    let worker_function = if cfg!(feature = "embassy") {
        quote! {
            #[allow(non_snake_case)]
            #[::medi_rs::embassy_executor::task]
            async fn #worker(mediator: &'static #name) {
                #worker_loop
            }
        }
    } else {
        quote! {
            #[allow(non_snake_case)]
            async fn #worker(mediator: &'static #name) {
                #worker_loop
            }
        }
    };
    let queue_type = if cfg!(feature = "embassy") {
        quote! { ::medi_rs::adapters::selected::EventQueue<#job, { #capacity }> }
    } else {
        quote! { ::medi_rs::adapters::selected::EventQueue<#job> }
    };
    let task_cancellations = (0..task_spawns.len()).map(|index| {
        let field = format_ident!("task_shutdown_{index}");
        quote! { self.#field.cancel(); }
    });
    let start = generate_event_start(resource_tuple, &worker, task_spawns);
    EventSupport {
        job: quote! { #[allow(non_camel_case_types)] enum #job { #(#variants,)* Shutdown } },
        field: quote! { event_queue: #queue_type, },
        publish_routes: quote! { #(#publish_routes)* },
        publish_method: quote! { /// Enqueue an event for later worker dispatch.
        pub async fn publish<E>(&self, event: E) -> ::medi_rs::Result<()> where E: ::medi_rs::StaticPublish<Self> { event.publish(self).await }
        /// Attempt to enqueue an event without waiting for queue capacity.
        ///
        /// On failure, the returned error contains the event for retry or
        /// application-specific overload handling.
        pub fn try_publish<E>(&self, event: E) -> core::result::Result<(), ::medi_rs::TryPublishError<E>> where E: ::medi_rs::StaticTryPublish<Self> { event.try_publish(self) }
        /// Stop accepting events, drain accepted events, and wait for all event
        /// workers and registered runtime tasks to return.
        pub async fn shutdown(&self) -> ::medi_rs::Result<()> {
            if self.lifecycle.request_shutdown() {
                #(#task_cancellations)*
                ::medi_rs::EventQueue::close(&self.event_queue).await;
                let event_workers = Self::EVENT_WORKERS;
                for _ in 0..event_workers {
                    ::medi_rs::EventQueue::publish_internal(&self.event_queue, #job::Shutdown).await?;
                }
            }
            self.lifecycle.wait().await;
            Ok(())
        } },
        worker: quote! { #worker_function impl #name { #start } },
    }
}

pub(crate) fn generate_task_only_start(
    has_events: bool,
    has_tasks: bool,
    name: &Ident,
    resource_tuple: &proc_macro2::TokenStream,
    task_spawns: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    if has_events || !has_tasks {
        return quote! {};
    }
    let task_cancellations = (0..task_spawns.len()).map(|index| {
        let field = format_ident!("task_shutdown_{index}");
        quote! { self.#field.cancel(); }
    });
    if cfg!(feature = "embassy") {
        quote! { impl #name {
            /// Start the registered Embassy tasks.
            ///
            /// Returns [`::medi_rs::StartError::AlreadyStarted`] without
            /// spawning work when this mediator was already started.
            pub fn start(&'static self, spawner: ::medi_rs::embassy_executor::Spawner) -> core::result::Result<(), ::medi_rs::StartError> where #resource_tuple: Sync {
                self.lifecycle.start()?;
                #(#task_spawns)*
                Ok(())
            }
            /// Signal registered runtime tasks and wait for them to return.
            pub async fn shutdown(&self) -> ::medi_rs::Result<()> {
                if self.lifecycle.request_shutdown() {
                    #(#task_cancellations)*
                }
                self.lifecycle.wait().await;
                Ok(())
            }
        } }
    } else {
        quote! { impl #name {
            /// Start the registered runtime tasks.
            ///
            /// Returns [`::medi_rs::StartError::AlreadyStarted`] without
            /// spawning work when this mediator was already started.
            pub fn start(&'static self) -> core::result::Result<(), ::medi_rs::StartError> where #resource_tuple: Sync {
                self.lifecycle.start()?;
                #(#task_spawns)*
                Ok(())
            }
            /// Signal registered runtime tasks and wait for them to return.
            pub async fn shutdown(&self) -> ::medi_rs::Result<()> {
                if self.lifecycle.request_shutdown() {
                    #(#task_cancellations)*
                }
                self.lifecycle.wait().await;
                Ok(())
            }
        } }
    }
}
