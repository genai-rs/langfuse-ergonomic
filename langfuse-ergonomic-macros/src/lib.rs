//! The `#[observe]` attribute of [langfuse-ergonomic](https://docs.rs/langfuse-ergonomic).
//!
//! Use it through `langfuse_ergonomic::observe` (feature `macros`, on by default); this crate
//! only holds the procedural macro. The expansion calls `::langfuse_ergonomic::__private`, so
//! the two crates are released together with the same version. See
//! [`langfuse_ergonomic::observe`](https://docs.rs/langfuse-ergonomic/latest/langfuse_ergonomic/observe/)
//! for the options and the runtime behaviour.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, FnArg, Ident, ItemFn, LitBool, LitStr, Pat, ReturnType, Type};

/// Trace a function as a Langfuse observation.
///
/// Works on `async fn` and synchronous functions, free or in `impl` blocks. Options (all
/// optional):
///
/// - `name = "..."`: observation name (default: the function name)
/// - `as_type = "..."`: `span` (default), `generation`, `event`, `agent`, `tool`, `chain`,
///   `retriever`, `evaluator`, `embedding` or `guardrail`
/// - `capture_input = false`, `capture_output = false`
/// - `skip(arg, ..)`: arguments not to record (`self` never is)
/// - `err`: on `Err`, record the error as the output too
/// - `level = "DEBUG" | "DEFAULT" | "WARNING" | "ERROR"`
/// - `model = expr`, `user_id = expr`, `session_id = expr`, `tags = expr`, `metadata = expr`
/// - `stream` (returns `impl Stream`) or `stream = "iterator"` (returns `impl Iterator`)
///
/// See `langfuse_ergonomic::observe` for the semantics.
#[proc_macro_attribute]
pub fn observe(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item2: TokenStream2 = item.clone().into();
    let function = match syn::parse::<ItemFn>(item) {
        Ok(function) => function,
        Err(_) => {
            let error = syn::Error::new(
                Span::call_site(),
                "#[observe] applies to functions with a body (`fn` or `async fn`)",
            );
            return with_error(error, item2);
        }
    };
    let mut options = Options::default();
    let parser = syn::meta::parser(|meta| options.parse(meta));
    if let Err(error) = syn::parse::Parser::parse(parser, attr) {
        return with_error(error, item2);
    }
    match expand(options, function) {
        Ok(tokens) => tokens.into(),
        Err(error) => with_error(error, item2),
    }
}

/// The error, and the function unchanged (so that the error is the only one reported).
fn with_error(error: syn::Error, item: TokenStream2) -> TokenStream {
    let error = error.to_compile_error();
    quote!(#error #item).into()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StreamKind {
    Stream,
    Iterator,
}

#[derive(Default)]
struct Options {
    name: Option<LitStr>,
    as_type: Option<Ident>,
    capture_input: Option<bool>,
    capture_output: Option<bool>,
    skip: Option<Vec<Ident>>,
    err: bool,
    level: Option<Ident>,
    model: Option<Expr>,
    user_id: Option<Expr>,
    session_id: Option<Expr>,
    tags: Option<Expr>,
    metadata: Option<Expr>,
    stream: Option<StreamKind>,
}


const TYPES: [(&str, &str); 10] = [
    ("span", "Span"),
    ("generation", "Generation"),
    ("event", "Event"),
    ("agent", "Agent"),
    ("tool", "Tool"),
    ("chain", "Chain"),
    ("retriever", "Retriever"),
    ("evaluator", "Evaluator"),
    ("embedding", "Embedding"),
    ("guardrail", "Guardrail"),
];

const LEVELS: [(&str, &str); 6] = [
    ("debug", "Debug"),
    ("default", "Default"),
    ("info", "Default"),
    ("warning", "Warning"),
    ("warn", "Warning"),
    ("error", "Error"),
];

fn set_once<T>(slot: &mut Option<T>, value: T, span: Span, name: &str) -> syn::Result<()> {
    if slot.is_some() {
        return Err(syn::Error::new(span, format!("duplicate `{name}` option")));
    }
    *slot = Some(value);
    Ok(())
}

impl Options {
    fn parse(&mut self, meta: syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
        let span = meta.path.span();
        let Some(key) = meta.path.get_ident().map(ToString::to_string) else {
            return Err(meta.error("unknown #[observe] option"));
        };
        match key.as_str() {
            "name" => {
                let value: LitStr = meta.value()?.parse()?;
                set_once(&mut self.name, value, span, "name")
            }
            "as_type" => {
                let value: LitStr = meta.value()?.parse()?;
                let wanted = value.value().to_ascii_lowercase();
                let Some((_, variant)) = TYPES.iter().find(|(name, _)| *name == wanted) else {
                    let names: Vec<&str> = TYPES.iter().map(|(name, _)| *name).collect();
                    return Err(syn::Error::new(
                        value.span(),
                        format!(
                            "unknown observation type; expected one of: {}",
                            names.join(", ")
                        ),
                    ));
                };
                set_once(
                    &mut self.as_type,
                    Ident::new(variant, value.span()),
                    span,
                    "as_type",
                )
            }
            "capture_input" | "capture_output" => {
                let value: LitBool = meta.value()?.parse()?;
                let slot = if key == "capture_input" {
                    &mut self.capture_input
                } else {
                    &mut self.capture_output
                };
                set_once(slot, value.value, span, &key)
            }
            "skip" => {
                let mut names = Vec::new();
                meta.parse_nested_meta(|nested| {
                    match nested.path.get_ident() {
                        Some(ident) => names.push(ident.clone()),
                        None => return Err(nested.error("expected an argument name")),
                    }
                    Ok(())
                })?;
                set_once(&mut self.skip, names, span, "skip")
            }
            "err" => {
                if meta.input.peek(syn::Token![=]) {
                    return Err(meta.error("`err` takes no value"));
                }
                if self.err {
                    return Err(syn::Error::new(span, "duplicate `err` option"));
                }
                self.err = true;
                Ok(())
            }
            "level" => {
                let value: LitStr = meta.value()?.parse()?;
                let wanted = value.value().to_ascii_lowercase();
                let Some((_, variant)) = LEVELS.iter().find(|(name, _)| *name == wanted) else {
                    return Err(syn::Error::new(
                        value.span(),
                        "unknown level; expected DEBUG, DEFAULT, WARNING or ERROR",
                    ));
                };
                set_once(
                    &mut self.level,
                    Ident::new(variant, value.span()),
                    span,
                    "level",
                )
            }
            "model" => set_once(&mut self.model, meta.value()?.parse()?, span, "model"),
            "user_id" => set_once(&mut self.user_id, meta.value()?.parse()?, span, "user_id"),
            "session_id" => set_once(
                &mut self.session_id,
                meta.value()?.parse()?,
                span,
                "session_id",
            ),
            "tags" => set_once(&mut self.tags, meta.value()?.parse()?, span, "tags"),
            "metadata" => set_once(&mut self.metadata, meta.value()?.parse()?, span, "metadata"),
            "stream" => {
                let kind = if meta.input.peek(syn::Token![=]) {
                    let value: LitStr = meta.value()?.parse()?;
                    match value.value().as_str() {
                        "stream" => StreamKind::Stream,
                        "iterator" | "iter" => StreamKind::Iterator,
                        _ => {
                            return Err(syn::Error::new(
                                value.span(),
                                "expected `stream` or `stream = \"iterator\"`",
                            ))
                        }
                    }
                } else {
                    StreamKind::Stream
                };
                set_once(&mut self.stream, kind, span, "stream")
            }
            _ => Err(meta.error(format!(
                "unknown #[observe] option `{key}`; expected name, as_type, capture_input, \
                 capture_output, skip, err, level, model, user_id, session_id, tags, metadata \
                 or stream"
            ))),
        }
    }
}

/// Whether a type mentions `impl Trait` (which cannot annotate a `let`).
fn has_impl_trait(ty: &Type) -> bool {
    struct Finder(bool);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_type_impl_trait(&mut self, _: &'ast syn::TypeImplTrait) {
            self.0 = true;
        }
    }
    let mut finder = Finder(false);
    finder.visit_type(ty);
    finder.0
}

fn expand(options: Options, function: ItemFn) -> syn::Result<TokenStream2> {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = function;

    if let Some(constness) = &sig.constness {
        return Err(syn::Error::new(
            constness.span(),
            "#[observe] cannot be used on a `const fn`",
        ));
    }

    // The arguments to record.
    let mut arg_names: Vec<Ident> = Vec::new();
    for input in &sig.inputs {
        if let FnArg::Typed(typed) = input {
            if let Pat::Ident(pat) = &*typed.pat {
                arg_names.push(pat.ident.clone());
            }
        }
    }
    let skip = options.skip.unwrap_or_default();
    for name in &skip {
        if !arg_names.iter().any(|arg| arg == name) {
            return Err(syn::Error::new(
                name.span(),
                format!("`skip`: the function has no argument `{name}`"),
            ));
        }
    }
    let captured: Vec<&Ident> = arg_names.iter().filter(|arg| !skip.contains(arg)).collect();

    let krate = quote!(::langfuse_ergonomic::__private);
    let name = options
        .name
        .map_or_else(|| sig.ident.to_string(), |name| name.value());
    let kind = options
        .as_type
        .unwrap_or_else(|| Ident::new("Span", Span::call_site()));
    let level = match options.level {
        Some(level) => quote!(::core::option::Option::Some(#krate::ObservationLevel::#level)),
        None => quote!(::core::option::Option::None),
    };
    let capture_input = options.capture_input.unwrap_or(true);
    let capture_output = options.capture_output.unwrap_or(true);
    let capture_err = options.err;

    let input = if capture_input && !captured.is_empty() {
        let names = captured.iter().map(ToString::to_string);
        quote! {
            #krate::set_input(__lf_o, ::std::vec![
                #( (#names, (&&&#krate::Capture(&#captured)).__lf_capture()) ),*
            ]);
        }
    } else {
        quote!()
    };
    let model = options.model.map(|model| quote!(__lf_o.set_model(#model);));
    let user_id = options
        .user_id
        .map(|user_id| quote!(__lf_o.set_user_id(#user_id);));
    let session_id = options
        .session_id
        .map(|session_id| quote!(__lf_o.set_session_id(#session_id);));
    let tags = options
        .tags
        .map(|tags| quote!(#krate::set_tags(__lf_o, #tags);));
    let metadata = options.metadata.map(|metadata| {
        quote!(#krate::set_metadata(__lf_o, (&&&#krate::Capture(&(#metadata))).__lf_capture());)
    });

    let prelude = quote! {
        #[allow(unused_imports)]
        use #krate::{
            CaptureDebug as _, CaptureNone as _, CaptureSerialize as _, ErrMsgDebug as _,
            ErrMsgDisplay as _, ErrMsgNone as _, OutcomeAny as _, OutcomeResult as _,
        };
        const __LF_META: #krate::Meta = #krate::Meta {
            name: #name,
            kind: #krate::ObservationType::#kind,
            level: #level,
            capture_input: #capture_input,
            capture_output: #capture_output,
            capture_err: #capture_err,
        };
        let __lf_obs = #krate::start(&__LF_META);
        if let ::core::option::Option::Some(__lf_o) = #krate::recording(&__lf_obs) {
            #input
            #model
            #user_id
            #session_id
            #tags
            #metadata
        }
    };

    let ok_capture = if capture_output {
        quote!(__lf_split.ok.and_then(|__lf_v| (&&&#krate::Capture(__lf_v)).__lf_capture()))
    } else {
        quote!(::core::option::Option::None)
    };
    let err_capture = if capture_err {
        quote!((&&&#krate::Capture(__lf_e)).__lf_capture())
    } else {
        quote!(::core::option::Option::None)
    };
    // Record a return value `__lf_r` on `__lf_o`.
    let record_body = quote! {
        let __lf_split = (&&#krate::Outcome(__lf_r)).__lf_split();
        let __lf_ok = #ok_capture;
        let __lf_err = __lf_split
            .err
            .map(|__lf_e| ((&&&#krate::ErrMsg(__lf_e)).__lf_msg(), #err_capture));
        #krate::record(__lf_o, &__LF_META, __lf_ok, __lf_err);
    };
    let item_capture = quote! {
        |__lf_r: &_| {
            let __lf_split = (&&#krate::Outcome(__lf_r)).__lf_split();
            #krate::ItemCapture {
                value: #ok_capture,
                error: __lf_split.err.map(|__lf_e| (&&&#krate::ErrMsg(__lf_e)).__lf_msg()),
            }
        }
    };

    let ret_type: Option<Type> = match &sig.output {
        ReturnType::Default => Some(syn::parse_quote!(())),
        ReturnType::Type(_, ty) if has_impl_trait(ty) => None,
        ReturnType::Type(_, ty) => Some((**ty).clone()),
    };

    let body = if sig.asyncness.is_some() {
        let fake_return = ret_type.as_ref().map(|ty| {
            quote! {
                #[allow(
                    unknown_lints,
                    unreachable_code,
                    clippy::diverging_sub_expression,
                    clippy::let_unit_value,
                    clippy::unreachable,
                    clippy::empty_loop,
                    clippy::needless_return
                )]
                if false {
                    let __lf_fake: #ty = loop {};
                    return __lf_fake;
                }
            }
        });
        let future = quote! {
            async move {
                #fake_return
                #block
            }
        };
        match options.stream {
            None => quote! {
                #prelude
                #krate::instrument(
                    __lf_obs,
                    #future,
                    |__lf_o: &::langfuse_ergonomic::observe::Observation, __lf_r: &_| {
                        #record_body
                    },
                )
                .await
            },
            Some(kind) => {
                let wrap = stream_wrap(&krate, kind, &item_capture, capture_output);
                quote! {
                    #prelude
                    let __lf_ret = #krate::instrument_open(
                        ::core::clone::Clone::clone(&__lf_obs),
                        #future,
                    )
                    .await;
                    #wrap
                }
            }
        }
    } else {
        let annotation = ret_type.as_ref().map(|ty| quote!(-> #ty));
        let call = quote! {
            #[allow(clippy::redundant_closure_call)]
            let __lf_ret = (move || #annotation #block)();
        };
        match options.stream {
            None => quote! {
                #prelude
                let __lf_scope = #krate::scope(::core::clone::Clone::clone(&__lf_obs));
                #call
                if let ::core::option::Option::Some(__lf_o) = #krate::recording(&__lf_obs) {
                    let __lf_r = &__lf_ret;
                    #record_body
                }
                ::core::mem::drop(__lf_scope);
                __lf_ret
            },
            Some(kind) => {
                let wrap = stream_wrap(&krate, kind, &item_capture, capture_output);
                quote! {
                    #prelude
                    let __lf_scope = #krate::open_scope(::core::clone::Clone::clone(&__lf_obs));
                    #call
                    ::core::mem::drop(__lf_scope);
                    #wrap
                }
            }
        }
    };

    let sig_tokens = sig.to_token_stream();
    Ok(quote! {
        #(#attrs)*
        #vis #sig_tokens {
            #body
        }
    })
}

fn stream_wrap(
    krate: &TokenStream2,
    kind: StreamKind,
    item_capture: &TokenStream2,
    capture_output: bool,
) -> TokenStream2 {
    match kind {
        StreamKind::Stream => quote! {
            #krate::observe_stream(__lf_obs, __lf_ret, #item_capture, #capture_output)
        },
        StreamKind::Iterator => quote! {
            #krate::observe_iter(__lf_obs, __lf_ret, #item_capture, #capture_output)
        },
    }
}
