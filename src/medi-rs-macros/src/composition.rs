//! Mediator-composition parsing, validation, and code generation.

use crate::generate::{
    collect_event_routes, collect_resource_types, collect_tasks, generate_command_routes, generate_constructor,
    generate_event_support, generate_task_only_start, generate_task_spawns, generate_task_workers,
};
use crate::manifest::ModuleManifest;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Expr, Ident, Path, Result as SynResult, Token, Type, Visibility, braced, bracketed, parse_macro_input};

fn nested_tuple_type(resources: &[Type]) -> proc_macro2::TokenStream {
    resources.iter().rev().fold(quote! { () }, |tail, resource| {
        quote! { (#resource, #tail) }
    })
}

fn nested_tuple_value(resources: &[Ident]) -> proc_macro2::TokenStream {
    resources.iter().rev().fold(quote! { () }, |tail, resource| {
        quote! { (#resource, #tail) }
    })
}

struct CompositionMarkerInput {
    vis: Visibility,
    name: Ident,
    event_queue_capacity: Expr,
    event_workers: Expr,
    modules: Vec<ModuleManifest>,
    decorators: Vec<Path>,
    count: proc_macro2::TokenStream,
}

impl Parse for CompositionMarkerInput {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let vis: Visibility = input.parse()?;
        input.parse::<Token![struct]>()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![;]>()?;

        let capacity_key: Ident = input.parse()?;
        if capacity_key != "event_queue_capacity" {
            return Err(syn::Error::new(capacity_key.span(), "expected `event_queue_capacity`"));
        }
        input.parse::<Token![:]>()?;
        let event_queue_capacity = input.parse()?;
        input.parse::<Token![;]>()?;

        let workers_key: Ident = input.parse()?;
        if workers_key != "event_workers" {
            return Err(syn::Error::new(workers_key.span(), "expected `event_workers`"));
        }
        input.parse::<Token![:]>()?;
        let event_workers = input.parse()?;
        input.parse::<Token![;]>()?;

        let modules_key: Ident = input.parse()?;
        if modules_key != "modules" {
            return Err(syn::Error::new(modules_key.span(), "expected `modules`"));
        }
        input.parse::<Token![:]>()?;

        let modules;
        bracketed!(modules in input);
        let mut parsed_modules = Vec::new();
        while !modules.is_empty() {
            let manifest;
            braced!(manifest in modules);
            parsed_modules.push(manifest.parse()?);
            if !modules.is_empty() {
                modules.parse::<Token![,]>()?;
            }
        }
        input.parse::<Token![;]>()?;

        let decorators_key: Ident = input.parse()?;
        if decorators_key != "decorators" {
            return Err(syn::Error::new(decorators_key.span(), "expected `decorators`"));
        }
        input.parse::<Token![:]>()?;
        let decorators_body;
        bracketed!(decorators_body in input);
        let mut decorators = Vec::new();
        while !decorators_body.is_empty() {
            decorators.push(decorators_body.parse()?);
            if !decorators_body.is_empty() {
                decorators_body.parse::<Token![,]>()?;
            }
        }
        input.parse::<Token![;]>()?;

        let count_key: Ident = input.parse()?;
        if count_key != "count" {
            return Err(syn::Error::new(count_key.span(), "expected `count`"));
        }
        input.parse::<Token![:]>()?;
        let count;
        bracketed!(count in input);
        let count: proc_macro2::TokenStream = count.parse()?;
        input.parse::<Token![;]>()?;

        Ok(Self {
            vis,
            name,
            event_queue_capacity,
            event_workers,
            modules: parsed_modules,
            decorators,
            count,
        })
    }
}

fn combine_error(errors: &mut Option<syn::Error>, error: syn::Error) {
    if let Some(errors) = errors {
        errors.combine(error);
    } else {
        *errors = Some(error);
    }
}

fn validate_unique_registrations(modules: &[ModuleManifest]) -> SynResult<()> {
    let mut registrations = std::collections::HashMap::new();
    let mut resources = std::collections::HashMap::new();
    let mut errors = None;
    for module in modules {
        for command in &module.commands {
            let request = &command.request;
            let key = quote!(#request).to_string();
            if registrations.insert(key.clone(), request.span()).is_some() {
                combine_error(
                    &mut errors,
                    syn::Error::new(request.span(), format!("duplicate command registration for `{key}`")),
                );
            }
        }
        for resource in &module.resources {
            let key = quote!(#resource).to_string();
            if resources.insert(key.clone(), resource.span()).is_some() {
                combine_error(
                    &mut errors,
                    syn::Error::new(resource.span(), format!("duplicate resource registration for `{key}`")),
                );
            }
        }
    }
    errors.map_or(Ok(()), Err)
}

/// Temporary composition endpoint. It validates the full registration graph and emits static dispatch code.
pub fn mediator_composition_marker_inner(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as CompositionMarkerInput);
    if let Err(error) = validate_unique_registrations(&input.modules) {
        return error.into_compile_error().into();
    }

    let event_routes = collect_event_routes(&input.modules);
    let has_events = !event_routes.is_empty();
    if has_events && !cfg!(any(feature = "tokio", feature = "wasm", feature = "embassy")) {
        return syn::Error::new(
            input.name.span(),
            "event mediators require the `tokio`, `wasm`, or `embassy` feature",
        )
        .into_compile_error()
        .into();
    }
    let resource_types = collect_resource_types(&input.modules);
    let resource_tuple = nested_tuple_type(&resource_types);
    let resource_names: Vec<_> = (0..resource_types.len())
        .map(|index| format_ident!("resource_{index}"))
        .collect();
    let resource_values = nested_tuple_value(&resource_names);
    let tasks = collect_tasks(&input.modules);
    let task_workers = generate_task_workers(&tasks, &input.name);
    let task_spawns = generate_task_spawns(&tasks);
    let command_routes = generate_command_routes(&input.modules, &input.name, &input.decorators);
    let constructor = generate_constructor(
        &resource_types,
        &resource_names,
        &resource_values,
        has_events,
        &input.event_queue_capacity,
    );
    let job_name = format_ident!("{}EventJob", input.name);
    let event_support = has_events.then(|| {
        generate_event_support(
            &event_routes,
            &input.name,
            &job_name,
            &input.event_queue_capacity,
            &resource_tuple,
            &input.decorators,
            &task_spawns,
        )
    });
    let event_job = event_support
        .as_ref()
        .map_or_else(|| quote! {}, |support| support.job.clone());
    let event_field = event_support
        .as_ref()
        .map_or_else(|| quote! {}, |support| support.field.clone());
    let publish_routes = event_support
        .as_ref()
        .map_or_else(|| quote! {}, |support| support.publish_routes.clone());
    let publish_method = event_support
        .as_ref()
        .map_or_else(|| quote! {}, |support| support.publish_method.clone());
    let event_worker = event_support
        .as_ref()
        .map_or_else(|| quote! {}, |support| support.worker.clone());
    let task_only_start = generate_task_only_start(
        has_events,
        !tasks.is_empty(),
        &input.name,
        &resource_tuple,
        &task_spawns,
    );
    let vis = input.vis;
    let name = input.name;
    let capacity = input.event_queue_capacity;
    let workers = input.event_workers;
    let count = input.count;
    quote! {
        #event_job
        #vis struct #name { resources: #resource_tuple, #event_field }
        impl #name {
            #constructor
            /// Configured capacity for the generated event queue.
            pub const EVENT_QUEUE_CAPACITY: usize = #capacity;
            /// Number of event worker tasks started by [`Self::start`].
            pub const EVENT_WORKERS: usize = #workers;
            /// Number of manifests included in this composition.
            pub const MODULE_COUNT: usize = <[()]>::len(&[#count]);
            /// Send a command through its macro-generated static route.
            pub async fn send<C>(&self, command: C) -> core::result::Result<C::Response, C::Error> where C: ::medi_rs::StaticSendCommand<Self> { command.send(self).await }
            #publish_method
        }
        #(#command_routes)* #publish_routes #event_worker #(#task_workers)* #task_only_start
    }.into()
}
