initSidebarItems({"fn":[["align_of","Returns the [ABI]-required minimum alignment of a type."],["align_of_val","Returns the [ABI]-required minimum alignment of the type of the value that `val` points to."],["align_of_val_raw","Returns the [ABI]-required minimum alignment of the type of the value that `val` points to."],["discriminant","Returns a value uniquely identifying the enum variant in `v`."],["drop","Disposes of a value."],["forget","Takes ownership and \"forgets\" about the value without running its destructor."],["forget_unsized","Like [`forget`], but also accepts unsized values."],["min_align_of","Returns the [ABI]-required minimum alignment of a type."],["min_align_of_val","Returns the [ABI]-required minimum alignment of the type of the value that `val` points to."],["needs_drop","Returns `true` if dropping values of type `T` matters."],["replace","Moves `src` into the referenced `dest`, returning the previous `dest` value."],["size_of","Returns the size of a type in bytes."],["size_of_val","Returns the size of the pointed-to value in bytes."],["size_of_val_raw","Returns the size of the pointed-to value in bytes."],["swap","Swaps the values at two mutable locations, without deinitializing either one."],["take","Replaces `dest` with the default value of `T`, returning the previous `dest` value."],["transmute","Reinterprets the bits of a value of one type as another type."],["transmute_copy","Interprets `src` as having type `&U`, and then reads `src` without moving the contained value."],["uninitialized","Bypasses Rust's normal memory-initialization checks by pretending to produce a value of type `T`, while doing nothing at all."],["zeroed","Returns the value of type `T` represented by the all-zero byte-pattern."]],"struct":[["Discriminant","Opaque type representing the discriminant of an enum."],["ManuallyDrop","A wrapper to inhibit compiler from automatically calling `T`’s destructor."]],"union":[["MaybeUninit","A wrapper type to construct uninitialized instances of `T`."]]});