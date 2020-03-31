use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Expr, ExprArray, Ident, Path, Result, Token
};

fn find_expr_init_type(expr: Expr) -> Path {
    match expr {
        Expr::Struct(struct_) => struct_.path,
        Expr::Path(path) => path.path,
        _ => {
            panic!("Expected struct literal or path, got {:?}", expr);
        }
    }
}

///
pub struct WasmClientInput {
    exchange_idents: Vec<Path>,
    exchange_initializers: Vec<Expr>,
    query_collection: Path,
    extra_initializers: Vec<(Ident, Expr)>
}

impl Parse for WasmClientInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        struct TempInput {
            exchange_idents: Vec<Path>,
            exchange_initializers: Option<Vec<Expr>>,
            query_collection: Option<Path>,
            extra_initializers: Vec<(Ident, Expr)>
        }

        let mut input_struct = TempInput {
            extra_initializers: Vec::new(),
            exchange_idents: Vec::new(),
            query_collection: None,
            exchange_initializers: None
        };

        while !input.is_empty() {
            let mut lookahead = input.lookahead1();
            if !lookahead.peek(Ident) {
                return Err(lookahead.error());
            }
            let key = input.parse::<Ident>().unwrap();
            lookahead = input.lookahead1();
            if !lookahead.peek(Token![:]) {
                return Err(lookahead.error());
            }
            input.parse::<Token![:]>().unwrap();
            //lookahead = input.lookahead1();

            let key_str = key.to_string();

            if key_str == "queries" || key_str == "query_collection" {
                /*                if !lookahead.peek(Path) {
                    return Err(lookahead.error());
                }*/
                let type_ = input.parse::<Path>().unwrap();
                input_struct.query_collection = Some(type_);
            } else if key_str == "exchanges" {
                /*                if !lookahead.peek(ExprArray) {
                    return Err(lookahead.error());
                }*/
                let initializers: Vec<Expr> = input
                    .parse::<ExprArray>()
                    .unwrap()
                    .elems
                    .into_iter()
                    .collect();
                let idents: Vec<Path> = initializers
                    .iter()
                    .cloned()
                    .map(find_expr_init_type)
                    .collect();
                input_struct.exchange_initializers = Some(initializers);
                input_struct.exchange_idents = idents;
            } else {
                /*                if !lookahead.peek(Expr) {
                    return Err(lookahead.error());
                }*/
                let expr = input.parse::<Expr>().unwrap();
                input_struct.extra_initializers.push((key, expr));
            }

            if !input.is_empty() {
                lookahead = input.lookahead1();

                if !lookahead.peek(Token![,]) {
                    return Err(lookahead.error());
                }
                input.parse::<Token![,]>().unwrap();
            }
        }

        let query_collection = input_struct.query_collection.expect(
            r#"'queries' key must be present in the wasm_client macro.
             This should correspond to the 'Queries' enum generated
             in the root of the quries module."#
        );
        let exchange_initializers = input_struct.exchange_initializers.unwrap_or_else(Vec::new);

        Ok(WasmClientInput {
            exchange_idents: input_struct.exchange_idents,
            exchange_initializers,
            query_collection,
            extra_initializers: input_struct.extra_initializers
        })
    }
}

///
#[allow(clippy::cmp_owned)]
pub fn wasm_client(input: WasmClientInput) -> TokenStream {
    let exchange_ident = input.exchange_idents.iter().fold(
        quote!(::artemis::exchanges::DummyExchange),
        |current, item| quote!(<#item as ::artemis::ExchangeFactory<#current>>::Output)
    );
    let exchange_initializers = input
        .exchange_initializers
        .iter()
        .map(|initializer| quote!(with_exchange(#initializer)));
    let query_collection = input.query_collection;
    let url = input.extra_initializers.iter().find_map(|(key, init)| {
        if key.to_string() == "url" {
            Some(init)
        } else {
            None
        }
    });
    let extra_initializers: Vec<_> = input
        .extra_initializers
        .iter()
        .filter(|(key, _)| key.to_string() != "url")
        .map(|(key, value)| {
            let ident = Ident::new(&format!("with_{}", key.to_string()), Span::call_site());
            quote! { #ident(#value) }
        })
        .collect();
    let url = if let Some(url) = url {
        quote!(with_url(unsafe { options.url() }.unwrap_or_else(|| #url)))
    } else {
        quote!(unsafe { options.url() }.expect("Must provide a URL to the client"))
    };

    let tokens = quote! {
        #[wasm_bindgen]
        pub struct Client {
            inner: ::std::sync::Arc<::artemis::client::JsClient<#exchange_ident, #query_collection>>
        }

        #[wasm_bindgen(skip_typescript)]
        impl Client {
            #[wasm_bindgen(constructor)]
            pub fn new(options: &::artemis::wasm::JsClientOptions) -> Self {
                let mut inner_client = ::artemis::Client::builder(#url)
                    #(.#exchange_initializers)*
                    #(.#extra_initializers)*;

                if let Some(request_policy) = unsafe { options.request_policy() } {
                    inner_client = inner_client.with_request_policy(request_policy.into());
                }

                if let Some(headers) = unsafe { options.headers() } {
                    inner_client = inner_client.with_js_extra_headers(headers);
                }

                if let Some(fetch) = unsafe { options.fetch() } {
                    inner_client = inner_client.with_fetch(fetch);
                }

                Self {
                    inner: ::std::sync::Arc::new(
                        ::artemis::client::JsClient::<_, #query_collection>::new(inner_client.build())
                    )
                }
            }

            pub fn query(&self, query: #query_collection, variables: ::wasm_bindgen::JsValue, options: Option<::artemis::wasm::JsQueryOptions>)
             -> js_sys::Promise {
                let inner = self.inner.clone();
                wasm_bindgen_futures::future_to_promise(async move {
                    inner.query(query, variables, options).await
                })
            }

            pub fn subscribe(
                &self,
                query: #query_collection,
                variables: ::wasm_bindgen::JsValue,
                callback: ::js_sys::Function,
                options: Option<::artemis::wasm::JsQueryOptions>
            ) {
                self.inner.subscribe(query, variables, callback, options)
            }
        }
    };

    //println!("{}", tokens);

    tokens
}
