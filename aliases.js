var ALIASES = {};
ALIASES["ahash"] = {};
ALIASES["aho_corasick"] = {};
ALIASES["arc_swap"] = {};
ALIASES["artemis"] = {};
ALIASES["artemis_build"] = {};
ALIASES["artemis_codegen"] = {};
ALIASES["artemis_codegen_proc_macro"] = {};
ALIASES["artemis_normalized_cache"] = {};
ALIASES["artemis_test"] = {};
ALIASES["ascii"] = {};
ALIASES["ast_node"] = {};
ALIASES["async_trait"] = {};
ALIASES["atty"] = {};
ALIASES["backtrace"] = {};
ALIASES["backtrace_sys"] = {};
ALIASES["base64"] = {};
ALIASES["bincode"] = {};
ALIASES["bitflags"] = {};
ALIASES["byteorder"] = {};
ALIASES["bytes"] = {};
ALIASES["cfg_if"] = {};
ALIASES["combine"] = {};
ALIASES["const_random"] = {};
ALIASES["const_random_macro"] = {};
ALIASES["crossbeam_epoch"] = {};
ALIASES["crossbeam_utils"] = {};
ALIASES["darling"] = {};
ALIASES["darling_core"] = {};
ALIASES["darling_macro"] = {};
ALIASES["dashmap"] = {};
ALIASES["debug_unreachable"] = {};
ALIASES["dprint_core"] = {};
ALIASES["dprint_plugin_typescript"] = {};
ALIASES["dtoa"] = {};
ALIASES["either"] = {};
ALIASES["encoding_rs"] = {};
ALIASES["enum_kind"] = {};
ALIASES["failure"] = {};
ALIASES["failure_derive"] = {};
ALIASES["flurry"] = {};
ALIASES["fnv"] = {};
ALIASES["foreign_types"] = {};
ALIASES["foreign_types_shared"] = {};
ALIASES["from_variant"] = {};
ALIASES["futures"] = {};
ALIASES["futures_channel"] = {};
ALIASES["futures_core"] = {};
ALIASES["futures_executor"] = {};
ALIASES["futures_io"] = {};
ALIASES["futures_macro"] = {};
ALIASES["futures_sink"] = {};
ALIASES["futures_task"] = {};
ALIASES["futures_util"] = {};
ALIASES["fxhash"] = {};
ALIASES["getrandom"] = {};
ALIASES["graphql_parser"] = {};
ALIASES["h2"] = {};
ALIASES["hashbrown"] = {};
ALIASES["heck"] = {};
ALIASES["http"] = {};
ALIASES["http_body"] = {};
ALIASES["httparse"] = {};
ALIASES["hyper"] = {};
ALIASES["hyper_tls"] = {};
ALIASES["ident_case"] = {};
ALIASES["idna"] = {};
ALIASES["if_chain"] = {};
ALIASES["indexmap"] = {};
ALIASES["iovec"] = {};
ALIASES["itoa"] = {};
ALIASES["lazy_static"] = {};
ALIASES["libc"] = {};
ALIASES["lock_api"] = {};
ALIASES["log"] = {};
ALIASES["matches"] = {};
ALIASES["maybe_uninit"] = {};
ALIASES["memchr"] = {};
ALIASES["memoffset"] = {};
ALIASES["mime"] = {};
ALIASES["mime_guess"] = {};
ALIASES["mio"] = {};
ALIASES["mio_uds"] = {};
ALIASES["native_tls"] = {};
ALIASES["net2"] = {};
ALIASES["no_std_compat"] = {"?":[{'crate':'no_std_compat','ty':8,'name':'Try','desc':'A trait for customizing the behavior of the `?` operator.','p':'no_std_compat::ops'}],"memcpy":[{'crate':'no_std_compat','ty':5,'name':'copy_nonoverlapping','desc':'Copies `count * size_of::<T>()` bytes from `src` to `dst`.…','p':'no_std_compat::ptr'}],"memmove":[{'crate':'no_std_compat','ty':5,'name':'copy','desc':'Copies `count * size_of::<T>()` bytes from `src` to `dst`.…','p':'no_std_compat::ptr'}],"*":[{'crate':'no_std_compat','ty':8,'name':'Mul','desc':'The multiplication operator `*`.','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':8,'name':'MulAssign','desc':'The multiplication assignment operator `*=`.','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':8,'name':'Deref','desc':'Used for immutable dereferencing operations, like `*v`.','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':8,'name':'DerefMut','desc':'Used for mutable dereferencing operations, like in `*v =…','p':'no_std_compat::ops'}],">=":[{'crate':'no_std_compat','ty':8,'name':'Ord','desc':'Trait for types that form a total order.','p':'no_std_compat::cmp'},{'crate':'no_std_compat','ty':8,'name':'PartialOrd','desc':'Trait for values that can be compared for a sort-order.','p':'no_std_compat::cmp'},{'crate':'no_std_compat','ty':8,'name':'Ord','desc':'Trait for types that form a total order.','p':'no_std_compat::prelude::v1'},{'crate':'no_std_compat','ty':8,'name':'PartialOrd','desc':'Trait for values that can be compared for a sort-order.','p':'no_std_compat::prelude::v1'}],"..=":[{'crate':'no_std_compat','ty':3,'name':'RangeInclusive','desc':'A range bounded inclusively below and above (`start..=end`).','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':3,'name':'RangeToInclusive','desc':'A range only bounded inclusively above (`..=end`).','p':'no_std_compat::ops'}],"^=":[{'crate':'no_std_compat','ty':8,'name':'BitXorAssign','desc':'The bitwise XOR assignment operator `^=`.','p':'no_std_compat::ops'}],">":[{'crate':'no_std_compat','ty':8,'name':'Ord','desc':'Trait for types that form a total order.','p':'no_std_compat::cmp'},{'crate':'no_std_compat','ty':8,'name':'PartialOrd','desc':'Trait for values that can be compared for a sort-order.','p':'no_std_compat::cmp'},{'crate':'no_std_compat','ty':8,'name':'Ord','desc':'Trait for types that form a total order.','p':'no_std_compat::prelude::v1'},{'crate':'no_std_compat','ty':8,'name':'PartialOrd','desc':'Trait for values that can be compared for a sort-order.','p':'no_std_compat::prelude::v1'}],"<=":[{'crate':'no_std_compat','ty':8,'name':'Ord','desc':'Trait for types that form a total order.','p':'no_std_compat::cmp'},{'crate':'no_std_compat','ty':8,'name':'PartialOrd','desc':'Trait for values that can be compared for a sort-order.','p':'no_std_compat::cmp'},{'crate':'no_std_compat','ty':8,'name':'Ord','desc':'Trait for types that form a total order.','p':'no_std_compat::prelude::v1'},{'crate':'no_std_compat','ty':8,'name':'PartialOrd','desc':'Trait for values that can be compared for a sort-order.','p':'no_std_compat::prelude::v1'}],"^":[{'crate':'no_std_compat','ty':8,'name':'BitXor','desc':'The bitwise XOR operator `^`.','p':'no_std_compat::ops'}],"&=":[{'crate':'no_std_compat','ty':8,'name':'BitAndAssign','desc':'The bitwise AND assignment operator `&=`.','p':'no_std_compat::ops'}],"[]":[{'crate':'no_std_compat','ty':8,'name':'Index','desc':'Used for indexing operations (`container[index]`) in…','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':8,'name':'IndexMut','desc':'Used for indexing operations (`container[index]`) in…','p':'no_std_compat::ops'}],"+":[{'crate':'no_std_compat','ty':8,'name':'Add','desc':'The addition operator `+`.','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':8,'name':'AddAssign','desc':'The addition assignment operator `+=`.','p':'no_std_compat::ops'}],"%":[{'crate':'no_std_compat','ty':8,'name':'Rem','desc':'The remainder operator `%`.','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':8,'name':'RemAssign','desc':'The remainder assignment operator `%=`.','p':'no_std_compat::ops'}],">>=":[{'crate':'no_std_compat','ty':8,'name':'ShrAssign','desc':'The right shift assignment operator `>>=`.','p':'no_std_compat::ops'}],"/":[{'crate':'no_std_compat','ty':8,'name':'Div','desc':'The division operator `/`.','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':8,'name':'DivAssign','desc':'The division assignment operator `/=`.','p':'no_std_compat::ops'}],"[":[{'crate':'no_std_compat','ty':8,'name':'Index','desc':'Used for indexing operations (`container[index]`) in…','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':8,'name':'IndexMut','desc':'Used for indexing operations (`container[index]`) in…','p':'no_std_compat::ops'}],"..":[{'crate':'no_std_compat','ty':3,'name':'Range','desc':'A (half-open) range bounded inclusively below and…','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':3,'name':'RangeFrom','desc':'A range only bounded inclusively below (`start..`).','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':3,'name':'RangeFull','desc':'An unbounded range (`..`).','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':3,'name':'RangeTo','desc':'A range only bounded exclusively above (`..end`).','p':'no_std_compat::ops'}],"<<":[{'crate':'no_std_compat','ty':8,'name':'Shl','desc':'The left shift operator `<<`. Note that because this trait…','p':'no_std_compat::ops'}],"{}":[{'crate':'no_std_compat','ty':8,'name':'Display','desc':'Format trait for an empty format, `{}`.','p':'no_std_compat::fmt'},{'crate':'no_std_compat','ty':8,'name':'Display','desc':'Format trait for an empty format, `{}`.','p':'no_std_compat::fmt'}],"<":[{'crate':'no_std_compat','ty':8,'name':'Ord','desc':'Trait for types that form a total order.','p':'no_std_compat::cmp'},{'crate':'no_std_compat','ty':8,'name':'PartialOrd','desc':'Trait for values that can be compared for a sort-order.','p':'no_std_compat::cmp'},{'crate':'no_std_compat','ty':8,'name':'Ord','desc':'Trait for types that form a total order.','p':'no_std_compat::prelude::v1'},{'crate':'no_std_compat','ty':8,'name':'PartialOrd','desc':'Trait for values that can be compared for a sort-order.','p':'no_std_compat::prelude::v1'}],"*=":[{'crate':'no_std_compat','ty':8,'name':'MulAssign','desc':'The multiplication assignment operator `*=`.','p':'no_std_compat::ops'}],"&":[{'crate':'no_std_compat','ty':8,'name':'BitAnd','desc':'The bitwise AND operator `&`.','p':'no_std_compat::ops'}],"|=":[{'crate':'no_std_compat','ty':8,'name':'BitOrAssign','desc':'The bitwise OR assignment operator `|=`.','p':'no_std_compat::ops'}],"|":[{'crate':'no_std_compat','ty':8,'name':'BitOr','desc':'The bitwise OR operator `|`.','p':'no_std_compat::ops'}],"&*":[{'crate':'no_std_compat','ty':8,'name':'Deref','desc':'Used for immutable dereferencing operations, like `*v`.','p':'no_std_compat::ops'}],">>":[{'crate':'no_std_compat','ty':8,'name':'Shr','desc':'The right shift operator `>>`. Note that because this…','p':'no_std_compat::ops'}],"-=":[{'crate':'no_std_compat','ty':8,'name':'SubAssign','desc':'The subtraction assignment operator `-=`.','p':'no_std_compat::ops'}],"==":[{'crate':'no_std_compat','ty':8,'name':'PartialEq','desc':'Trait for equality comparisons which are partial…','p':'no_std_compat::cmp'},{'crate':'no_std_compat','ty':8,'name':'Eq','desc':'Trait for equality comparisons which are equivalence…','p':'no_std_compat::cmp'},{'crate':'no_std_compat','ty':8,'name':'Eq','desc':'Trait for equality comparisons which are equivalence…','p':'no_std_compat::prelude::v1'},{'crate':'no_std_compat','ty':8,'name':'PartialEq','desc':'Trait for equality comparisons which are partial…','p':'no_std_compat::prelude::v1'}],"!=":[{'crate':'no_std_compat','ty':8,'name':'PartialEq','desc':'Trait for equality comparisons which are partial…','p':'no_std_compat::cmp'},{'crate':'no_std_compat','ty':8,'name':'Eq','desc':'Trait for equality comparisons which are equivalence…','p':'no_std_compat::cmp'},{'crate':'no_std_compat','ty':8,'name':'Eq','desc':'Trait for equality comparisons which are equivalence…','p':'no_std_compat::prelude::v1'},{'crate':'no_std_compat','ty':8,'name':'PartialEq','desc':'Trait for equality comparisons which are partial…','p':'no_std_compat::prelude::v1'}],"-":[{'crate':'no_std_compat','ty':8,'name':'Neg','desc':'The unary negation operator `-`.','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':8,'name':'Sub','desc':'The subtraction operator `-`.','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':8,'name':'SubAssign','desc':'The subtraction assignment operator `-=`.','p':'no_std_compat::ops'}],"+=":[{'crate':'no_std_compat','ty':8,'name':'AddAssign','desc':'The addition assignment operator `+=`.','p':'no_std_compat::ops'}],"<<=":[{'crate':'no_std_compat','ty':8,'name':'ShlAssign','desc':'The left shift assignment operator `<<=`.','p':'no_std_compat::ops'}],"{:?}":[{'crate':'no_std_compat','ty':8,'name':'Debug','desc':'`?` formatting.','p':'no_std_compat::fmt'},{'crate':'no_std_compat','ty':8,'name':'Debug','desc':'`?` formatting.','p':'no_std_compat::fmt'}],"%=":[{'crate':'no_std_compat','ty':8,'name':'RemAssign','desc':'The remainder assignment operator `%=`.','p':'no_std_compat::ops'}],"/=":[{'crate':'no_std_compat','ty':8,'name':'DivAssign','desc':'The division assignment operator `/=`.','p':'no_std_compat::ops'}],"]":[{'crate':'no_std_compat','ty':8,'name':'Index','desc':'Used for indexing operations (`container[index]`) in…','p':'no_std_compat::ops'},{'crate':'no_std_compat','ty':8,'name':'IndexMut','desc':'Used for indexing operations (`container[index]`) in…','p':'no_std_compat::ops'}],};
ALIASES["num_bigint"] = {};
ALIASES["num_cpus"] = {};
ALIASES["num_integer"] = {};
ALIASES["num_traits"] = {};
ALIASES["once_cell"] = {};
ALIASES["openssl"] = {};
ALIASES["openssl_probe"] = {};
ALIASES["openssl_sys"] = {};
ALIASES["owning_ref"] = {};
ALIASES["parking_lot"] = {};
ALIASES["parking_lot_core"] = {};
ALIASES["percent_encoding"] = {};
ALIASES["phf_shared"] = {};
ALIASES["pin_project"] = {};
ALIASES["pin_project_internal"] = {};
ALIASES["pin_project_lite"] = {};
ALIASES["pin_utils"] = {};
ALIASES["pmutil"] = {};
ALIASES["precomputed_hash"] = {};
ALIASES["proc_macro2"] = {};
ALIASES["proc_macro_hack"] = {};
ALIASES["proc_macro_nested"] = {};
ALIASES["quote"] = {};
ALIASES["rand"] = {};
ALIASES["rand_chacha"] = {};
ALIASES["rand_core"] = {};
ALIASES["rand_hc"] = {};
ALIASES["rand_isaac"] = {};
ALIASES["rand_jitter"] = {};
ALIASES["rand_os"] = {};
ALIASES["rand_pcg"] = {};
ALIASES["rand_xorshift"] = {};
ALIASES["regex"] = {};
ALIASES["regex_syntax"] = {};
ALIASES["reqwest"] = {};
ALIASES["rustc_demangle"] = {};
ALIASES["ryu"] = {};
ALIASES["scoped_tls"] = {};
ALIASES["scopeguard"] = {};
ALIASES["serde"] = {};
ALIASES["serde_derive"] = {};
ALIASES["serde_json"] = {};
ALIASES["serde_urlencoded"] = {};
ALIASES["signal_hook_registry"] = {};
ALIASES["siphasher"] = {};
ALIASES["slab"] = {};
ALIASES["smallvec"] = {};
ALIASES["sourcemap"] = {};
ALIASES["stable_deref_trait"] = {};
ALIASES["stable_vec"] = {};
ALIASES["string_cache"] = {};
ALIASES["string_enum"] = {};
ALIASES["strsim"] = {};
ALIASES["swc_atoms"] = {};
ALIASES["swc_common"] = {};
ALIASES["swc_ecma_ast"] = {};
ALIASES["swc_ecma_parser"] = {};
ALIASES["swc_ecma_parser_macros"] = {};
ALIASES["swc_macros_common"] = {};
ALIASES["syn"] = {};
ALIASES["synstructure"] = {};
ALIASES["termcolor"] = {};
ALIASES["thread_local"] = {};
ALIASES["time"] = {};
ALIASES["tokio"] = {};
ALIASES["tokio_macros"] = {};
ALIASES["tokio_tls"] = {};
ALIASES["tokio_util"] = {};
ALIASES["tower_service"] = {};
ALIASES["try_lock"] = {};
ALIASES["unicase"] = {};
ALIASES["unicode_bidi"] = {};
ALIASES["unicode_normalization"] = {};
ALIASES["unicode_segmentation"] = {};
ALIASES["unicode_width"] = {};
ALIASES["unicode_xid"] = {};
ALIASES["unreachable"] = {};
ALIASES["url"] = {};
ALIASES["void"] = {};
ALIASES["want"] = {};
