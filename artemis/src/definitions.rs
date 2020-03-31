use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"

export type Maybe<T> = T | null | undefined;

export type Response<T> = { data: Maybe<T>, errors: Maybe<Error[]> }

export type Extensions = { [K: string]: any }

export type Error = {
    message: string,
    locations: Maybe<Location[]>,
    path: Maybe<PathFragment[]>,
    extensions: Maybe<Extensions>
}

export type PathFragment = string | number

export type Location = {
    line: number,
    column: number
}

export type ClientOptions = {
    url: Maybe<string>,
    headers: Maybe<() => Headers>,
    requestPolicy: Maybe<RequestPolicy>,
    fetch: (url: string, init: RequestInit) => Promise<any>
};
export type Headers = { [K: string]: string };
export enum RequestPolicy {
    CacheFirst = 1,
    CacheOnly = 2,
    NetworkOnly = 3,
    CacheAndNetwork = 4
}

export type QueryOptions = {
    url: Maybe<string>,
    headers: Maybe<() => Headers>,
    requestPolicy: Maybe<RequestPolicy>,
    extensions: Maybe<ExtensionMap>
};
export type ExtensionMap = { [K: string]: Extension };

/**
 * This corresponds to the Rust side Extension trait.
 * Any extension class will work here, it's just a semantic type.
 */
export type Extension = any;

export interface Client<Q> {
    new (options: ClientOptions): Client<Q>,
    query<V, R>(query: Q, variables: V, options: QueryOptions): Promise<R>,
    subscribe<V, R>(query: Q, variables: V, callback: (ok: R, err: any) => void, options: QueryOptions): void
}

"#;
