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
) -> proc_macro2::TokenStream {
    let configuration_check = quote! {
        assert!(#capacity > 0, "event_queue_capacity must be greater than zero");
    };
    match (resource_types.is_empty(), has_events) {
        (true, true) => quote! {
            /// Construct a mediator with no typed resources.
            pub fn new() -> Self {
                #configuration_check
                Self { resources: (), event_queue: ::medi_rs::EventQueue::new(Some(#capacity)) }
            }
        },
        (true, false) => quote! {
            /// Construct a mediator with no typed resources.
            pub const fn new() -> Self { Self { resources: () } }
        },
        (false, true) => quote! {
            /// Construct a mediator from its declared resource values.
            pub fn new((#(#resource_names,)*): (#(#resource_types,)*)) -> Self {
                #configuration_check
                Self { resources: #resource_values, event_queue: ::medi_rs::EventQueue::new(Some(#capacity)) }
            }
        },
        (false, false) => quote! {
            /// Construct a mediator from its declared resource values.
            pub fn new((#(#resource_names,)*): (#(#resource_types,)*)) -> Self {
                Self { resources: #resource_values }
            }
        },
    }
}

pub(crate) fn generate_task_workers(tasks: &[syn::Path], name: &Ident) -> Vec<proc_macro2::TokenStream> {
    tasks.iter().enumerate().map(|(index, task)| {
        let worker = format_ident!("medi_rs_task_{index}");
        let invoker = task_invoker_path(task);
        if cfg!(feature = "embassy") {
            quote! { #[::embassy_executor::task] async fn #worker(mediator: &'static #name) { #invoker(mediator, &mediator.resources).await; } }
        } else {
            quote! { async fn #worker(mediator: &'static #name) { #invoker(mediator, &mediator.resources).await; } }
        }
    }).collect()
}

pub(crate) fn generate_task_spawns(tasks: &[syn::Path]) -> Vec<proc_macro2::TokenStream> {
    tasks
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let worker = format_ident!("medi_rs_task_{index}");
            if cfg!(feature = "embassy") {
                quote! { spawner.spawn(#worker(self)).ok(); }
            } else {
                quote! { ::medi_rs::adapters::selected::spawn(#worker(self)); }
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
) -> Vec<proc_macro2::TokenStream> {
    routes
        .iter()
        .enumerate()
        .map(|(index, (_, handlers))| {
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
                quote! { let _ = #invocation; }
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
            /// Start the generated Embassy event worker.
            pub fn start(&'static self, spawner: ::embassy_executor::Spawner) where #resource_tuple: Sync {
                assert!(Self::EVENT_WORKERS > 0, "event_workers must be greater than zero");
                for _ in 0..Self::EVENT_WORKERS { spawner.spawn(#worker(self)).ok(); }
                #(#task_spawns)*
            }
        }
    } else {
        quote! {
            /// Start the generated event worker and registered runtime tasks.
            pub fn start(&'static self) where #resource_tuple: Sync {
                assert!(Self::EVENT_WORKERS > 0, "event_workers must be greater than zero");
                for _ in 0..Self::EVENT_WORKERS { ::medi_rs::adapters::selected::spawn(#worker(self)); }
                #(#task_spawns)*
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
    decorators: &[Path],
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
            } }
        })
        .collect();
    let dispatch_arms = generate_event_dispatch_arms(routes, job, decorators);
    let worker = format_ident!("medi_rs_event_worker");
    let worker_loop = quote! { loop { match ::medi_rs::EventQueue::recv(&mediator.event_queue).await {
        Ok(event) => match event { #(#dispatch_arms)* }, Err(_) => break,
    } } };
    let worker_function = if cfg!(feature = "embassy") {
        quote! { #[allow(non_snake_case)] #[::embassy_executor::task] async fn #worker(mediator: &'static #name) { #worker_loop } }
    } else {
        quote! { #[allow(non_snake_case)] async fn #worker(mediator: &'static #name) { #worker_loop } }
    };
    let queue_type = if cfg!(feature = "embassy") {
        quote! { ::medi_rs::adapters::selected::EventQueue<#job, { #capacity }> }
    } else {
        quote! { ::medi_rs::adapters::selected::EventQueue<#job> }
    };
    let start = generate_event_start(resource_tuple, &worker, task_spawns);
    EventSupport {
        job: quote! { #[allow(non_camel_case_types)] enum #job { #(#variants,)* } },
        field: quote! { event_queue: #queue_type, },
        publish_routes: quote! { #(#publish_routes)* },
        publish_method: quote! { /// Enqueue an event for later worker dispatch.
        pub async fn publish<E>(&self, event: E) -> ::medi_rs::Result<()> where E: ::medi_rs::StaticPublish<Self> { event.publish(self).await } },
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
    if cfg!(feature = "embassy") {
        quote! { impl #name { /// Start the registered Embassy tasks.
        pub fn start(&'static self, spawner: ::embassy_executor::Spawner) where #resource_tuple: Sync { #(#task_spawns)* } } }
    } else {
        quote! { impl #name { /// Start the registered runtime tasks.
        pub fn start(&'static self) where #resource_tuple: Sync { #(#task_spawns)* } } }
    }
}
