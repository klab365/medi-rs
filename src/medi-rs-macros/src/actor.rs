//! Code generation for module-owned actor mailboxes and routes.

use crate::manifest::{ActorModuleManifest, ModuleManifest};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Path, Type};

type ActorEventSubscribers = Vec<(Ident, Ident, Ident)>;
type GroupedActorEvents = Vec<(Type, ActorEventSubscribers)>;

pub(crate) struct ActorSupport {
    pub(crate) fields: TokenStream,
    pub(crate) initializers: TokenStream,
    pub(crate) jobs: TokenStream,
    pub(crate) workers: TokenStream,
    pub(crate) starts: Vec<TokenStream>,
    pub(crate) stops: Vec<TokenStream>,
    pub(crate) routes: TokenStream,
    pub(crate) publish_method: TokenStream,
}

fn last_type_ident(module_type: &Type) -> Option<&Ident> {
    match module_type {
        Type::Path(path) => path.path.segments.last().map(|segment| &segment.ident),
        _ => None,
    }
}

pub(crate) fn associated_method(handler: &Path, module_type: &Type) -> Option<Ident> {
    let module_ident = last_type_ident(module_type)?;
    let segments: Vec<_> = handler.segments.iter().collect();
    if segments.len() < 2 || segments[segments.len() - 2].ident != *module_ident {
        return None;
    }
    Some(segments.last()?.ident.clone())
}

fn variant_name(request: &Type) -> Ident {
    match request {
        Type::Path(path) => format_ident!("{}", path.path.segments.last().unwrap().ident),
        _ => format_ident!("Message"),
    }
}

fn event_variant_name(event: &Type, method: &Ident, index: usize) -> Ident {
    format_ident!("Event{}_{}_{index}", variant_name(event), method)
}

fn mailbox_field(module_type: &Type, actor_field: &Ident) -> Ident {
    let _ = module_type;
    format_ident!("{}_mailbox", actor_field)
}

fn reply_field(actor_field: &Ident) -> Ident {
    format_ident!("{}_replies", actor_field)
}

fn job_type(module_type: &Type) -> Ident {
    let module = last_type_ident(module_type).expect("actor module types must be paths");
    format_ident!("{}ActorJob", module)
}

fn reply_type(module_type: &Type) -> Ident {
    let module = last_type_ident(module_type).expect("actor module types must be paths");
    format_ident!("{}ActorReply", module)
}

pub(crate) fn generate_actor_support(
    modules: &[ModuleManifest],
    mediator: &Ident,
    actor_field_name: impl Fn(&Type) -> Ident,
) -> ActorSupport {
    let actors: Vec<_> = modules
        .iter()
        .enumerate()
        .filter_map(|(index, module)| module.actor.as_ref().map(|actor| (index, module, actor)))
        .collect();
    let mut fields = Vec::new();
    let mut initializers = Vec::new();
    let mut job_enums = Vec::new();
    let mut reply_enums = Vec::new();
    let mut workers = Vec::new();
    let mut starts = Vec::new();
    let mut stops = Vec::new();
    let mut routes = Vec::new();
    let mut actor_event_targets: Vec<(Type, Ident, Ident, Ident)> = Vec::new();

    for (actor_id, module, actor) in actors {
        let ActorModuleManifest {
            module_type,
            mailbox_capacity,
        } = actor;
        if last_type_ident(module_type).is_none() {
            continue;
        }
        let actor_field = actor_field_name(module_type);
        let mailbox = mailbox_field(module_type, &actor_field);
        let job = job_type(module_type);
        let reply_enum = reply_type(module_type);
        let capacity = mailbox_capacity;
        let command_routes: Vec<_> = module
            .commands
            .iter()
            .filter_map(|route| associated_method(&route.handler, module_type).map(|method| (route, method)))
            .collect();
        let command_variants: Vec<_> = command_routes
            .iter()
            .map(|(route, _)| variant_name(&route.request))
            .collect();
        let event_routes: Vec<_> = module
            .events
            .iter()
            .flat_map(|event| {
                event.handlers.iter().enumerate().filter_map(move |(index, handler)| {
                    associated_method(handler, module_type).map(|method| (event, method, index))
                })
            })
            .collect();
        let event_variants: Vec<_> = event_routes
            .iter()
            .map(|(event, method, index)| event_variant_name(&event.event, method, *index))
            .collect();
        for ((event, _, _), variant) in event_routes.iter().zip(&event_variants) {
            actor_event_targets.push((event.event.clone(), mailbox.clone(), job.clone(), variant.clone()));
        }

        let tokio_wasm_job_variants = command_routes.iter().zip(&command_variants).map(|((route, _), variant)| {
            let request = &route.request;
            let send_variant = format_ident!("Send{variant}");
            let post_variant = format_ident!("Post{variant}");
            quote! {
                #send_variant(#request, ::medi_rs::adapters::selected::ReplySender<core::result::Result<<#request as ::medi_rs::Command>::Response, <#request as ::medi_rs::Command>::Error>>),
                #post_variant(#request),
            }
        });
        let embassy_job_variants = command_routes
            .iter()
            .zip(&command_variants)
            .map(|((route, _), variant)| {
                let request = &route.request;
                let send_variant = format_ident!("Send{variant}");
                let post_variant = format_ident!("Post{variant}");
                quote! { #send_variant(#request, ::medi_rs::adapters::selected::ReplyKey), #post_variant(#request), }
            });
        let tokio_wasm_event_variants = event_routes
            .iter()
            .zip(&event_variants)
            .map(|((event, _, _), variant)| {
                let event_type = &event.event;
                quote! { #variant(#event_type), }
            });
        let embassy_event_variants = tokio_wasm_event_variants.clone();
        let reply_variants = command_routes.iter().zip(&command_variants).map(|((route, _), variant)| {
            let request = &route.request;
            quote! { #variant(core::result::Result<<#request as ::medi_rs::Command>::Response, <#request as ::medi_rs::Command>::Error>), }
        });
        job_enums.push(quote! {
            #[allow(non_camel_case_types)]
            enum #job {
                #(#tokio_wasm_job_variants)*
                #(#tokio_wasm_event_variants)*
                Shutdown,
            }
        });
        let embassy_job_enum = quote! {
            #[allow(non_camel_case_types)]
            enum #job {
                #(#embassy_job_variants)*
                #(#embassy_event_variants)*
                Shutdown,
            }
        };
        let reply_enum_definition = quote! {
            enum #reply_enum { #(#reply_variants)* }
        };
        reply_enums.push(reply_enum_definition.clone());

        let queue_ty = if cfg!(feature = "embassy") {
            quote! { ::medi_rs::adapters::selected::EventQueue<#job, { #capacity }> }
        } else {
            quote! { ::medi_rs::adapters::selected::EventQueue<#job> }
        };
        fields.push(quote! { #mailbox: #queue_ty, });
        initializers.push(quote! { #mailbox: ::medi_rs::EventQueue::new(Some(#capacity)), });
        if cfg!(feature = "embassy") {
            let reply_field = reply_field(&actor_field);
            fields.push(quote! { #reply_field: ::medi_rs::adapters::selected::ReplyPool<#reply_enum, { #capacity }>, });
            initializers.push(quote! { #reply_field: ::medi_rs::adapters::selected::ReplyPool::new(), });
            job_enums.pop();
            job_enums.push(embassy_job_enum);
        }

        let worker = format_ident!("medi_rs_actor_{}", actor_field);
        let mailbox_ref = quote! { &mediator.#mailbox };
        let ctx = format_ident!("context");
        let mut method_arms = Vec::new();
        for ((route, method), variant) in command_routes.iter().zip(&command_variants) {
            let request = &route.request;
            let invocation = quote! { module.#method(&#ctx, command).await };
            let send_variant = format_ident!("Send{variant}");
            let post_variant = format_ident!("Post{variant}");
            let send_arm = if cfg!(feature = "embassy") {
                let replies = reply_field(&actor_field);
                quote! {
                    #job::#send_variant(command, key) => {
                        let result = #invocation;
                        mediator.#replies.respond(key, #reply_enum::#variant(result));
                    }
                }
            } else {
                quote! {
                    #job::#send_variant(command, reply) => {
                        let result = #invocation;
                        let _ = reply.send(result);
                    }
                }
            };
            let post_arm = quote! {
                #job::#post_variant(command) => { let _ = #invocation; }
            };
            method_arms.push(send_arm);
            method_arms.push(post_arm);

            let result_type = quote! { core::result::Result<<#request as ::medi_rs::Command>::Response, <#request as ::medi_rs::Command>::Error> };
            let send_route = if cfg!(feature = "embassy") {
                let replies = reply_field(&actor_field);
                quote! {
                    impl ::medi_rs::StaticSendCommand<#mediator> for #request {
                        fn target_module() -> Option<usize> { Some(#actor_id) }
                        fn send(self, mediator: &#mediator) -> impl core::future::Future<Output = core::result::Result<Self::Response, ::medi_rs::SendError<Self::Error>>> + Send {
                            async move {
                                if !mediator.is_started() {
                                    return Err(::medi_rs::SendError::Dispatch(::medi_rs::DispatchError::ModuleUnavailable));
                                }
                                let ticket = mediator.#replies.reserve().ok_or(::medi_rs::SendError::Dispatch(::medi_rs::DispatchError::ReplyCapacityExhausted))?;
                                let key = ticket.key();
                                if ::medi_rs::adapters::selected::Mailbox::send(&mediator.#mailbox, #job::#send_variant(self, key)).await.is_err() {
                                    ticket.cancel_unsent();
                                    return Err(::medi_rs::SendError::Dispatch(::medi_rs::DispatchError::ModuleUnavailable));
                                }
                                match ticket.wait().await {
                                    #reply_enum::#variant(Ok(response)) => Ok(response),
                                    #reply_enum::#variant(Err(error)) => Err(::medi_rs::SendError::Handler(error)),
                                    _ => unreachable!("actor reply variant does not match its command"),
                                }
                            }
                        }
                    }
                }
            } else {
                quote! {
                    impl ::medi_rs::StaticSendCommand<#mediator> for #request {
                        fn target_module() -> Option<usize> { Some(#actor_id) }
                        fn send(self, mediator: &#mediator) -> impl core::future::Future<Output = core::result::Result<Self::Response, ::medi_rs::SendError<Self::Error>>> + Send {
                            async move {
                                if !mediator.is_started() {
                                    return Err(::medi_rs::SendError::Dispatch(::medi_rs::DispatchError::ModuleUnavailable));
                                }
                                let (reply, response) = ::medi_rs::adapters::selected::reply_channel::<#result_type>();
                                ::medi_rs::adapters::selected::Mailbox::send(&mediator.#mailbox, #job::#send_variant(self, reply)).await.map_err(::medi_rs::SendError::Dispatch)?;
                                match response.await {
                                    Ok(Ok(value)) => Ok(value),
                                    Ok(Err(error)) => Err(::medi_rs::SendError::Handler(error)),
                                    Err(_) => Err(::medi_rs::SendError::Dispatch(::medi_rs::DispatchError::ModuleUnavailable)),
                                }
                            }
                        }
                    }
                }
            };
            routes.push(send_route);

            routes.push(quote! {
                impl ::medi_rs::StaticPostCommand<#mediator> for #request {
                    fn post(self, mediator: &#mediator) -> core::result::Result<(), ::medi_rs::PostError<Self>> {
                        match ::medi_rs::adapters::selected::Mailbox::post(&mediator.#mailbox, #job::#post_variant(self)) {
                            Ok(()) => Ok(()),
                            Err(::medi_rs::PostError::MailboxFull(#job::#post_variant(command))) => Err(::medi_rs::PostError::MailboxFull(command)),
                            Err(::medi_rs::PostError::ModuleUnavailable(#job::#post_variant(command))) => Err(::medi_rs::PostError::ModuleUnavailable(command)),
                            Err(_) => unreachable!("actor mailbox returned a different job variant"),
                        }
                    }
                }
            });
        }

        for ((event, method, _), variant) in event_routes.iter().zip(&event_variants) {
            let event_type = &event.event;
            method_arms.push(quote! {
                #job::#variant(event) => { let _ = module.#method(&#ctx, event).await; }
            });
            let _ = event_type;
        }
        let method_arms = method_arms;
        let worker_body = quote! {
            let Some(mut module) = mediator.#actor_field.take() else {
                mediator.lifecycle.finish();
                return;
            };
            let #ctx = ::medi_rs::MediContext::__new(mediator, #actor_id);
            loop {
                match ::medi_rs::adapters::selected::Mailbox::recv(#mailbox_ref).await {
                    Ok(#job::Shutdown) | Err(_) => break,
                    Ok(job) => match job { #(#method_arms,)* #job::Shutdown => break },
                }
            }
            mediator.lifecycle.finish();
        };
        if cfg!(feature = "embassy") {
            workers.push(quote! {
                #[::medi_rs::embassy_executor::task]
                async fn #worker(mediator: &'static #mediator) { #worker_body }
            });
            starts.push(quote! {
                self.lifecycle.begin();
                if let Ok(token) = #worker(self) { spawner.spawn(token); } else { self.lifecycle.finish(); }
            });
        } else {
            workers.push(quote! { async fn #worker(mediator: &'static #mediator) { #worker_body } });
            starts.push(quote! {
                self.lifecycle.begin();
                ::medi_rs::adapters::selected::spawn(#worker(self));
            });
        }
        stops.push(quote! {
            let _ = ::medi_rs::adapters::selected::Mailbox::stop(
                &self.#mailbox,
                #job::Shutdown,
            ).await;
        });
    }

    let mut grouped_events: GroupedActorEvents = Vec::new();
    for (event_type, mailbox, job, variant) in actor_event_targets {
        let key = quote!(#event_type).to_string();
        if let Some((_, targets)) = grouped_events
            .iter_mut()
            .find(|(registered, _)| quote!(#registered).to_string() == key)
        {
            targets.push((mailbox, job, variant));
        } else {
            grouped_events.push((event_type, vec![(mailbox, job, variant)]));
        }
    }
    let actor_publish_routes = grouped_events.iter().map(|(event, targets)| {
        let enqueues = targets.iter().map(|(mailbox, job, variant)| quote! {
            ::medi_rs::adapters::selected::Mailbox::post(
                &mediator.#mailbox,
                #job::#variant(self.clone()),
            ).map_err(|_| ::medi_rs::Error::EventPublishingError)?;
        });
        quote! {
            impl ::medi_rs::StaticPublish<#mediator> for #event
            where
                #event: Clone + Send + 'static,
            {
                fn publish(self, mediator: &#mediator) -> impl core::future::Future<Output = ::medi_rs::Result<()>> + Send {
                    async move { #(#enqueues)* Ok(()) }
                }
            }
        }
    });
    let publish_method = (!grouped_events.is_empty()).then(|| {
        quote! {
            /// Enqueue an event for its actor-module handlers.
            pub async fn publish<E>(&self, event: E) -> ::medi_rs::Result<()>
            where E: ::medi_rs::StaticPublish<Self> { event.publish(self).await }
        }
    });

    ActorSupport {
        fields: quote! { #(#fields)* },
        initializers: quote! { #(#initializers)* },
        jobs: quote! { #(#reply_enums)* #(#job_enums)* },
        workers: quote! { #(#workers)* },
        starts,
        stops,
        routes: quote! { #(#routes)* #(#actor_publish_routes)* },
        publish_method: publish_method.unwrap_or_else(|| quote! {}),
    }
}
