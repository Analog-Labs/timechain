pub mod types {
    use alloy_primitives::{Address, U256};
    use alloy_sol_types::{sol, Eip712Domain, SolCall, SolStruct};
    use sha3::{Digest, Keccak256};
    const EIP712_NAME: &str = "Analog Gateway Contract";
    const EIP712_VERSION: &str = "0.1.0";
    pub type Eip712Bytes = [u8; 66];
    #[allow(dead_code)]
    pub type Eip712Hash = [u8; 32];
    fn eip712_domain_separator(
        chain_id: u64,
        gateway_contract: Address,
    ) -> Eip712Domain {
        Eip712Domain {
            name: Some(EIP712_NAME.into()),
            version: Some(EIP712_VERSION.into()),
            chain_id: Some(U256::from(chain_id)),
            verifying_contract: Some(gateway_contract),
            salt: None,
        }
    }
    /**```solidity
struct TssKey { uint8 yParity; uint256 xCoord; }
```*/
    #[allow(non_camel_case_types, non_snake_case)]
    pub struct TssKey {
        pub yParity: u8,
        pub xCoord: ::alloy_sol_types::private::U256,
    }
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::clone::Clone for TssKey {
        #[inline]
        fn clone(&self) -> TssKey {
            TssKey {
                yParity: ::core::clone::Clone::clone(&self.yParity),
                xCoord: ::core::clone::Clone::clone(&self.xCoord),
            }
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::fmt::Debug for TssKey {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "TssKey",
                "yParity",
                &self.yParity,
                "xCoord",
                &&self.xCoord,
            )
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::default::Default for TssKey {
        #[inline]
        fn default() -> TssKey {
            TssKey {
                yParity: ::core::default::Default::default(),
                xCoord: ::core::default::Default::default(),
            }
        }
    }
    #[allow(non_camel_case_types, non_snake_case)]
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for TssKey {}
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::cmp::PartialEq for TssKey {
        #[inline]
        fn eq(&self, other: &TssKey) -> bool {
            self.yParity == other.yParity && self.xCoord == other.xCoord
        }
    }
    #[allow(non_camel_case_types, non_snake_case)]
    #[automatically_derived]
    impl ::core::marker::StructuralEq for TssKey {}
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::cmp::Eq for TssKey {
        #[inline]
        #[doc(hidden)]
        #[no_coverage]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<u8>;
            let _: ::core::cmp::AssertParamIsEq<::alloy_sol_types::private::U256>;
        }
    }
    #[allow(non_camel_case_types, non_snake_case, clippy::style)]
    const _: () = {
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (
            ::alloy_sol_types::sol_data::Uint<8>,
            ::alloy_sol_types::sol_data::Uint<256>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (u8, ::alloy_sol_types::private::U256);
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<TssKey> for UnderlyingRustTuple<'_> {
            fn from(value: TssKey) -> Self {
                (value.yParity, value.xCoord)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for TssKey {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    yParity: tuple.0,
                    xCoord: tuple.1,
                }
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolValue for TssKey {
            type SolType = Self;
        }
        #[automatically_derived]
        impl ::alloy_sol_types::private::SolTypeValue<Self> for TssKey {
            fn stv_to_tokens(&self) -> <Self as ::alloy_sol_types::SolType>::Token<'_> {
                (
                    <::alloy_sol_types::sol_data::Uint<
                        8,
                    > as ::alloy_sol_types::SolType>::tokenize(&self.yParity),
                    <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::SolType>::tokenize(&self.xCoord),
                )
            }
            #[inline]
            fn stv_abi_encoded_size(&self) -> usize {
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::abi_encoded_size(&tuple)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> ::alloy_sol_types::Word {
                <Self as ::alloy_sol_types::SolStruct>::eip712_hash_struct(self)
            }
            #[inline]
            fn stv_abi_encode_packed_to(
                &self,
                out: &mut ::alloy_sol_types::private::Vec<u8>,
            ) {
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::abi_encode_packed_to(&tuple, out)
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolType for TssKey {
            type RustType = Self;
            type Token<'a> = <UnderlyingSolTuple<
                'a,
            > as ::alloy_sol_types::SolType>::Token<'a>;
            const ENCODED_SIZE: Option<usize> = <UnderlyingSolTuple<
                '_,
            > as ::alloy_sol_types::SolType>::ENCODED_SIZE;
            #[inline]
            fn sol_type_name() -> ::alloy_sol_types::private::Cow<'static, str> {
                ::alloy_sol_types::private::Cow::Borrowed(
                    <Self as ::alloy_sol_types::SolStruct>::NAME,
                )
            }
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::valid_token(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                let tuple = <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::detokenize(token);
                <Self as ::core::convert::From<UnderlyingRustTuple<'_>>>::from(tuple)
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolStruct for TssKey {
            const NAME: &'static str = "TssKey";
            #[inline]
            fn eip712_root_type() -> ::alloy_sol_types::private::Cow<'static, str> {
                ::alloy_sol_types::private::Cow::Borrowed(
                    "TssKey(uint8 yParity,uint256 xCoord)",
                )
            }
            fn eip712_components() -> ::alloy_sol_types::private::Vec<
                ::alloy_sol_types::private::Cow<'static, str>,
            > {
                ::alloy_sol_types::private::Vec::new()
            }
            #[inline]
            fn eip712_encode_type() -> ::alloy_sol_types::private::Cow<'static, str> {
                <Self as ::alloy_sol_types::SolStruct>::eip712_root_type()
            }
            fn eip712_encode_data(&self) -> ::alloy_sol_types::private::Vec<u8> {
                [
                    <::alloy_sol_types::sol_data::Uint<
                        8,
                    > as ::alloy_sol_types::SolType>::eip712_data_word(&self.yParity)
                        .0,
                    <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::SolType>::eip712_data_word(&self.xCoord)
                        .0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::EventTopic for TssKey {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <::alloy_sol_types::sol_data::Uint<
                        8,
                    > as ::alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.yParity,
                    )
                    + <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.xCoord,
                    )
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut ::alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(
                    <Self as ::alloy_sol_types::EventTopic>::topic_preimage_length(rust),
                );
                <::alloy_sol_types::sol_data::Uint<
                    8,
                > as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.yParity,
                    out,
                );
                <::alloy_sol_types::sol_data::Uint<
                    256,
                > as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.xCoord,
                    out,
                );
            }
            #[inline]
            fn encode_topic(
                rust: &Self::RustType,
            ) -> ::alloy_sol_types::abi::token::WordToken {
                let mut out = ::alloy_sol_types::private::Vec::new();
                <Self as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    rust,
                    &mut out,
                );
                ::alloy_sol_types::abi::token::WordToken(
                    ::alloy_sol_types::private::keccak256(out),
                )
            }
        }
    };
    /**```solidity
struct UpdateKeysMessage { TssKey[] revoke; TssKey[] register; }
```*/
    #[allow(non_camel_case_types, non_snake_case)]
    pub struct UpdateKeysMessage {
        pub revoke: ::alloy_sol_types::private::Vec<
            <TssKey as ::alloy_sol_types::SolType>::RustType,
        >,
        pub register: ::alloy_sol_types::private::Vec<
            <TssKey as ::alloy_sol_types::SolType>::RustType,
        >,
    }
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::clone::Clone for UpdateKeysMessage {
        #[inline]
        fn clone(&self) -> UpdateKeysMessage {
            UpdateKeysMessage {
                revoke: ::core::clone::Clone::clone(&self.revoke),
                register: ::core::clone::Clone::clone(&self.register),
            }
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::fmt::Debug for UpdateKeysMessage {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "UpdateKeysMessage",
                "revoke",
                &self.revoke,
                "register",
                &&self.register,
            )
        }
    }
    #[allow(non_camel_case_types, non_snake_case)]
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for UpdateKeysMessage {}
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::cmp::PartialEq for UpdateKeysMessage {
        #[inline]
        fn eq(&self, other: &UpdateKeysMessage) -> bool {
            self.revoke == other.revoke && self.register == other.register
        }
    }
    #[allow(non_camel_case_types, non_snake_case)]
    #[automatically_derived]
    impl ::core::marker::StructuralEq for UpdateKeysMessage {}
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::cmp::Eq for UpdateKeysMessage {
        #[inline]
        #[doc(hidden)]
        #[no_coverage]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<
                ::alloy_sol_types::private::Vec<
                    <TssKey as ::alloy_sol_types::SolType>::RustType,
                >,
            >;
            let _: ::core::cmp::AssertParamIsEq<
                ::alloy_sol_types::private::Vec<
                    <TssKey as ::alloy_sol_types::SolType>::RustType,
                >,
            >;
        }
    }
    #[allow(non_camel_case_types, non_snake_case, clippy::style)]
    const _: () = {
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (
            ::alloy_sol_types::sol_data::Array<TssKey>,
            ::alloy_sol_types::sol_data::Array<TssKey>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            ::alloy_sol_types::private::Vec<
                <TssKey as ::alloy_sol_types::SolType>::RustType,
            >,
            ::alloy_sol_types::private::Vec<
                <TssKey as ::alloy_sol_types::SolType>::RustType,
            >,
        );
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UpdateKeysMessage> for UnderlyingRustTuple<'_> {
            fn from(value: UpdateKeysMessage) -> Self {
                (value.revoke, value.register)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for UpdateKeysMessage {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    revoke: tuple.0,
                    register: tuple.1,
                }
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolValue for UpdateKeysMessage {
            type SolType = Self;
        }
        #[automatically_derived]
        impl ::alloy_sol_types::private::SolTypeValue<Self> for UpdateKeysMessage {
            fn stv_to_tokens(&self) -> <Self as ::alloy_sol_types::SolType>::Token<'_> {
                (
                    <::alloy_sol_types::sol_data::Array<
                        TssKey,
                    > as ::alloy_sol_types::SolType>::tokenize(&self.revoke),
                    <::alloy_sol_types::sol_data::Array<
                        TssKey,
                    > as ::alloy_sol_types::SolType>::tokenize(&self.register),
                )
            }
            #[inline]
            fn stv_abi_encoded_size(&self) -> usize {
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::abi_encoded_size(&tuple)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> ::alloy_sol_types::Word {
                <Self as ::alloy_sol_types::SolStruct>::eip712_hash_struct(self)
            }
            #[inline]
            fn stv_abi_encode_packed_to(
                &self,
                out: &mut ::alloy_sol_types::private::Vec<u8>,
            ) {
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::abi_encode_packed_to(&tuple, out)
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolType for UpdateKeysMessage {
            type RustType = Self;
            type Token<'a> = <UnderlyingSolTuple<
                'a,
            > as ::alloy_sol_types::SolType>::Token<'a>;
            const ENCODED_SIZE: Option<usize> = <UnderlyingSolTuple<
                '_,
            > as ::alloy_sol_types::SolType>::ENCODED_SIZE;
            #[inline]
            fn sol_type_name() -> ::alloy_sol_types::private::Cow<'static, str> {
                ::alloy_sol_types::private::Cow::Borrowed(
                    <Self as ::alloy_sol_types::SolStruct>::NAME,
                )
            }
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::valid_token(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                let tuple = <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::detokenize(token);
                <Self as ::core::convert::From<UnderlyingRustTuple<'_>>>::from(tuple)
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolStruct for UpdateKeysMessage {
            const NAME: &'static str = "UpdateKeysMessage";
            #[inline]
            fn eip712_root_type() -> ::alloy_sol_types::private::Cow<'static, str> {
                ::alloy_sol_types::private::Cow::Borrowed(
                    "UpdateKeysMessage(TssKey[] revoke,TssKey[] register)",
                )
            }
            fn eip712_components() -> ::alloy_sol_types::private::Vec<
                ::alloy_sol_types::private::Cow<'static, str>,
            > {
                let mut components = ::alloy_sol_types::private::Vec::with_capacity(2);
                components
                    .push(<TssKey as ::alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(
                        <TssKey as ::alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(<TssKey as ::alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(
                        <TssKey as ::alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
            }
            fn eip712_encode_data(&self) -> ::alloy_sol_types::private::Vec<u8> {
                [
                    <::alloy_sol_types::sol_data::Array<
                        TssKey,
                    > as ::alloy_sol_types::SolType>::eip712_data_word(&self.revoke)
                        .0,
                    <::alloy_sol_types::sol_data::Array<
                        TssKey,
                    > as ::alloy_sol_types::SolType>::eip712_data_word(&self.register)
                        .0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::EventTopic for UpdateKeysMessage {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <::alloy_sol_types::sol_data::Array<
                        TssKey,
                    > as ::alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.revoke,
                    )
                    + <::alloy_sol_types::sol_data::Array<
                        TssKey,
                    > as ::alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.register,
                    )
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut ::alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(
                    <Self as ::alloy_sol_types::EventTopic>::topic_preimage_length(rust),
                );
                <::alloy_sol_types::sol_data::Array<
                    TssKey,
                > as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.revoke,
                    out,
                );
                <::alloy_sol_types::sol_data::Array<
                    TssKey,
                > as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.register,
                    out,
                );
            }
            #[inline]
            fn encode_topic(
                rust: &Self::RustType,
            ) -> ::alloy_sol_types::abi::token::WordToken {
                let mut out = ::alloy_sol_types::private::Vec::new();
                <Self as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    rust,
                    &mut out,
                );
                ::alloy_sol_types::abi::token::WordToken(
                    ::alloy_sol_types::private::keccak256(out),
                )
            }
        }
    };
    /**```solidity
struct GmpMessage { bytes32 source; uint128 srcNetwork; address dest; uint128 destNetwork; uint256 gasLimit; uint256 salt; bytes data; }
```*/
    #[allow(non_camel_case_types, non_snake_case)]
    pub struct GmpMessage {
        pub source: ::alloy_sol_types::private::FixedBytes<32>,
        pub srcNetwork: u128,
        pub dest: ::alloy_sol_types::private::Address,
        pub destNetwork: u128,
        pub gasLimit: ::alloy_sol_types::private::U256,
        pub salt: ::alloy_sol_types::private::U256,
        pub data: ::alloy_sol_types::private::Vec<u8>,
    }
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::clone::Clone for GmpMessage {
        #[inline]
        fn clone(&self) -> GmpMessage {
            GmpMessage {
                source: ::core::clone::Clone::clone(&self.source),
                srcNetwork: ::core::clone::Clone::clone(&self.srcNetwork),
                dest: ::core::clone::Clone::clone(&self.dest),
                destNetwork: ::core::clone::Clone::clone(&self.destNetwork),
                gasLimit: ::core::clone::Clone::clone(&self.gasLimit),
                salt: ::core::clone::Clone::clone(&self.salt),
                data: ::core::clone::Clone::clone(&self.data),
            }
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::fmt::Debug for GmpMessage {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "source",
                "srcNetwork",
                "dest",
                "destNetwork",
                "gasLimit",
                "salt",
                "data",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.source,
                &self.srcNetwork,
                &self.dest,
                &self.destNetwork,
                &self.gasLimit,
                &self.salt,
                &&self.data,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "GmpMessage",
                names,
                values,
            )
        }
    }
    #[allow(non_camel_case_types, non_snake_case)]
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for GmpMessage {}
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::cmp::PartialEq for GmpMessage {
        #[inline]
        fn eq(&self, other: &GmpMessage) -> bool {
            self.source == other.source && self.srcNetwork == other.srcNetwork
                && self.dest == other.dest && self.destNetwork == other.destNetwork
                && self.gasLimit == other.gasLimit && self.salt == other.salt
                && self.data == other.data
        }
    }
    #[allow(non_camel_case_types, non_snake_case)]
    #[automatically_derived]
    impl ::core::marker::StructuralEq for GmpMessage {}
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::cmp::Eq for GmpMessage {
        #[inline]
        #[doc(hidden)]
        #[no_coverage]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<
                ::alloy_sol_types::private::FixedBytes<32>,
            >;
            let _: ::core::cmp::AssertParamIsEq<u128>;
            let _: ::core::cmp::AssertParamIsEq<::alloy_sol_types::private::Address>;
            let _: ::core::cmp::AssertParamIsEq<::alloy_sol_types::private::U256>;
            let _: ::core::cmp::AssertParamIsEq<::alloy_sol_types::private::U256>;
            let _: ::core::cmp::AssertParamIsEq<::alloy_sol_types::private::Vec<u8>>;
        }
    }
    #[allow(non_camel_case_types, non_snake_case, clippy::style)]
    const _: () = {
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (
            ::alloy_sol_types::sol_data::FixedBytes<32>,
            ::alloy_sol_types::sol_data::Uint<128>,
            ::alloy_sol_types::sol_data::Address,
            ::alloy_sol_types::sol_data::Uint<128>,
            ::alloy_sol_types::sol_data::Uint<256>,
            ::alloy_sol_types::sol_data::Uint<256>,
            ::alloy_sol_types::sol_data::Bytes,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            ::alloy_sol_types::private::FixedBytes<32>,
            u128,
            ::alloy_sol_types::private::Address,
            u128,
            ::alloy_sol_types::private::U256,
            ::alloy_sol_types::private::U256,
            ::alloy_sol_types::private::Vec<u8>,
        );
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<GmpMessage> for UnderlyingRustTuple<'_> {
            fn from(value: GmpMessage) -> Self {
                (
                    value.source,
                    value.srcNetwork,
                    value.dest,
                    value.destNetwork,
                    value.gasLimit,
                    value.salt,
                    value.data,
                )
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for GmpMessage {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    source: tuple.0,
                    srcNetwork: tuple.1,
                    dest: tuple.2,
                    destNetwork: tuple.3,
                    gasLimit: tuple.4,
                    salt: tuple.5,
                    data: tuple.6,
                }
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolValue for GmpMessage {
            type SolType = Self;
        }
        #[automatically_derived]
        impl ::alloy_sol_types::private::SolTypeValue<Self> for GmpMessage {
            fn stv_to_tokens(&self) -> <Self as ::alloy_sol_types::SolType>::Token<'_> {
                (
                    <::alloy_sol_types::sol_data::FixedBytes<
                        32,
                    > as ::alloy_sol_types::SolType>::tokenize(&self.source),
                    <::alloy_sol_types::sol_data::Uint<
                        128,
                    > as ::alloy_sol_types::SolType>::tokenize(&self.srcNetwork),
                    <::alloy_sol_types::sol_data::Address as ::alloy_sol_types::SolType>::tokenize(
                        &self.dest,
                    ),
                    <::alloy_sol_types::sol_data::Uint<
                        128,
                    > as ::alloy_sol_types::SolType>::tokenize(&self.destNetwork),
                    <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::SolType>::tokenize(&self.gasLimit),
                    <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::SolType>::tokenize(&self.salt),
                    <::alloy_sol_types::sol_data::Bytes as ::alloy_sol_types::SolType>::tokenize(
                        &self.data,
                    ),
                )
            }
            #[inline]
            fn stv_abi_encoded_size(&self) -> usize {
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::abi_encoded_size(&tuple)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> ::alloy_sol_types::Word {
                <Self as ::alloy_sol_types::SolStruct>::eip712_hash_struct(self)
            }
            #[inline]
            fn stv_abi_encode_packed_to(
                &self,
                out: &mut ::alloy_sol_types::private::Vec<u8>,
            ) {
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::abi_encode_packed_to(&tuple, out)
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolType for GmpMessage {
            type RustType = Self;
            type Token<'a> = <UnderlyingSolTuple<
                'a,
            > as ::alloy_sol_types::SolType>::Token<'a>;
            const ENCODED_SIZE: Option<usize> = <UnderlyingSolTuple<
                '_,
            > as ::alloy_sol_types::SolType>::ENCODED_SIZE;
            #[inline]
            fn sol_type_name() -> ::alloy_sol_types::private::Cow<'static, str> {
                ::alloy_sol_types::private::Cow::Borrowed(
                    <Self as ::alloy_sol_types::SolStruct>::NAME,
                )
            }
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::valid_token(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                let tuple = <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::detokenize(token);
                <Self as ::core::convert::From<UnderlyingRustTuple<'_>>>::from(tuple)
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolStruct for GmpMessage {
            const NAME: &'static str = "GmpMessage";
            #[inline]
            fn eip712_root_type() -> ::alloy_sol_types::private::Cow<'static, str> {
                ::alloy_sol_types::private::Cow::Borrowed(
                    "GmpMessage(bytes32 source,uint128 srcNetwork,address dest,uint128 destNetwork,uint256 gasLimit,uint256 salt,bytes data)",
                )
            }
            fn eip712_components() -> ::alloy_sol_types::private::Vec<
                ::alloy_sol_types::private::Cow<'static, str>,
            > {
                ::alloy_sol_types::private::Vec::new()
            }
            #[inline]
            fn eip712_encode_type() -> ::alloy_sol_types::private::Cow<'static, str> {
                <Self as ::alloy_sol_types::SolStruct>::eip712_root_type()
            }
            fn eip712_encode_data(&self) -> ::alloy_sol_types::private::Vec<u8> {
                [
                    <::alloy_sol_types::sol_data::FixedBytes<
                        32,
                    > as ::alloy_sol_types::SolType>::eip712_data_word(&self.source)
                        .0,
                    <::alloy_sol_types::sol_data::Uint<
                        128,
                    > as ::alloy_sol_types::SolType>::eip712_data_word(&self.srcNetwork)
                        .0,
                    <::alloy_sol_types::sol_data::Address as ::alloy_sol_types::SolType>::eip712_data_word(
                            &self.dest,
                        )
                        .0,
                    <::alloy_sol_types::sol_data::Uint<
                        128,
                    > as ::alloy_sol_types::SolType>::eip712_data_word(&self.destNetwork)
                        .0,
                    <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::SolType>::eip712_data_word(&self.gasLimit)
                        .0,
                    <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::SolType>::eip712_data_word(&self.salt)
                        .0,
                    <::alloy_sol_types::sol_data::Bytes as ::alloy_sol_types::SolType>::eip712_data_word(
                            &self.data,
                        )
                        .0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::EventTopic for GmpMessage {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <::alloy_sol_types::sol_data::FixedBytes<
                        32,
                    > as ::alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.source,
                    )
                    + <::alloy_sol_types::sol_data::Uint<
                        128,
                    > as ::alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.srcNetwork,
                    )
                    + <::alloy_sol_types::sol_data::Address as ::alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.dest,
                    )
                    + <::alloy_sol_types::sol_data::Uint<
                        128,
                    > as ::alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.destNetwork,
                    )
                    + <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.gasLimit,
                    )
                    + <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.salt,
                    )
                    + <::alloy_sol_types::sol_data::Bytes as ::alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.data,
                    )
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut ::alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(
                    <Self as ::alloy_sol_types::EventTopic>::topic_preimage_length(rust),
                );
                <::alloy_sol_types::sol_data::FixedBytes<
                    32,
                > as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.source,
                    out,
                );
                <::alloy_sol_types::sol_data::Uint<
                    128,
                > as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.srcNetwork,
                    out,
                );
                <::alloy_sol_types::sol_data::Address as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.dest,
                    out,
                );
                <::alloy_sol_types::sol_data::Uint<
                    128,
                > as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.destNetwork,
                    out,
                );
                <::alloy_sol_types::sol_data::Uint<
                    256,
                > as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.gasLimit,
                    out,
                );
                <::alloy_sol_types::sol_data::Uint<
                    256,
                > as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.salt,
                    out,
                );
                <::alloy_sol_types::sol_data::Bytes as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.data,
                    out,
                );
            }
            #[inline]
            fn encode_topic(
                rust: &Self::RustType,
            ) -> ::alloy_sol_types::abi::token::WordToken {
                let mut out = ::alloy_sol_types::private::Vec::new();
                <Self as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    rust,
                    &mut out,
                );
                ::alloy_sol_types::abi::token::WordToken(
                    ::alloy_sol_types::private::keccak256(out),
                )
            }
        }
    };
    /**```solidity
struct Signature { uint256 xCoord; uint256 e; uint256 s; }
```*/
    #[allow(non_camel_case_types, non_snake_case)]
    pub struct Signature {
        pub xCoord: ::alloy_sol_types::private::U256,
        pub e: ::alloy_sol_types::private::U256,
        pub s: ::alloy_sol_types::private::U256,
    }
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::clone::Clone for Signature {
        #[inline]
        fn clone(&self) -> Signature {
            Signature {
                xCoord: ::core::clone::Clone::clone(&self.xCoord),
                e: ::core::clone::Clone::clone(&self.e),
                s: ::core::clone::Clone::clone(&self.s),
            }
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::fmt::Debug for Signature {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Signature",
                "xCoord",
                &self.xCoord,
                "e",
                &self.e,
                "s",
                &&self.s,
            )
        }
    }
    #[allow(non_camel_case_types, non_snake_case)]
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Signature {}
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::cmp::PartialEq for Signature {
        #[inline]
        fn eq(&self, other: &Signature) -> bool {
            self.xCoord == other.xCoord && self.e == other.e && self.s == other.s
        }
    }
    #[allow(non_camel_case_types, non_snake_case)]
    #[automatically_derived]
    impl ::core::marker::StructuralEq for Signature {}
    #[automatically_derived]
    #[allow(non_camel_case_types, non_snake_case)]
    impl ::core::cmp::Eq for Signature {
        #[inline]
        #[doc(hidden)]
        #[no_coverage]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<::alloy_sol_types::private::U256>;
            let _: ::core::cmp::AssertParamIsEq<::alloy_sol_types::private::U256>;
            let _: ::core::cmp::AssertParamIsEq<::alloy_sol_types::private::U256>;
        }
    }
    #[allow(non_camel_case_types, non_snake_case, clippy::style)]
    const _: () = {
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (
            ::alloy_sol_types::sol_data::Uint<256>,
            ::alloy_sol_types::sol_data::Uint<256>,
            ::alloy_sol_types::sol_data::Uint<256>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            ::alloy_sol_types::private::U256,
            ::alloy_sol_types::private::U256,
            ::alloy_sol_types::private::U256,
        );
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<Signature> for UnderlyingRustTuple<'_> {
            fn from(value: Signature) -> Self {
                (value.xCoord, value.e, value.s)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for Signature {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    xCoord: tuple.0,
                    e: tuple.1,
                    s: tuple.2,
                }
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolValue for Signature {
            type SolType = Self;
        }
        #[automatically_derived]
        impl ::alloy_sol_types::private::SolTypeValue<Self> for Signature {
            fn stv_to_tokens(&self) -> <Self as ::alloy_sol_types::SolType>::Token<'_> {
                (
                    <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::SolType>::tokenize(&self.xCoord),
                    <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::SolType>::tokenize(&self.e),
                    <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::SolType>::tokenize(&self.s),
                )
            }
            #[inline]
            fn stv_abi_encoded_size(&self) -> usize {
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::abi_encoded_size(&tuple)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> ::alloy_sol_types::Word {
                <Self as ::alloy_sol_types::SolStruct>::eip712_hash_struct(self)
            }
            #[inline]
            fn stv_abi_encode_packed_to(
                &self,
                out: &mut ::alloy_sol_types::private::Vec<u8>,
            ) {
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::abi_encode_packed_to(&tuple, out)
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolType for Signature {
            type RustType = Self;
            type Token<'a> = <UnderlyingSolTuple<
                'a,
            > as ::alloy_sol_types::SolType>::Token<'a>;
            const ENCODED_SIZE: Option<usize> = <UnderlyingSolTuple<
                '_,
            > as ::alloy_sol_types::SolType>::ENCODED_SIZE;
            #[inline]
            fn sol_type_name() -> ::alloy_sol_types::private::Cow<'static, str> {
                ::alloy_sol_types::private::Cow::Borrowed(
                    <Self as ::alloy_sol_types::SolStruct>::NAME,
                )
            }
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::valid_token(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                let tuple = <UnderlyingSolTuple<
                    '_,
                > as ::alloy_sol_types::SolType>::detokenize(token);
                <Self as ::core::convert::From<UnderlyingRustTuple<'_>>>::from(tuple)
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolStruct for Signature {
            const NAME: &'static str = "Signature";
            #[inline]
            fn eip712_root_type() -> ::alloy_sol_types::private::Cow<'static, str> {
                ::alloy_sol_types::private::Cow::Borrowed(
                    "Signature(uint256 xCoord,uint256 e,uint256 s)",
                )
            }
            fn eip712_components() -> ::alloy_sol_types::private::Vec<
                ::alloy_sol_types::private::Cow<'static, str>,
            > {
                ::alloy_sol_types::private::Vec::new()
            }
            #[inline]
            fn eip712_encode_type() -> ::alloy_sol_types::private::Cow<'static, str> {
                <Self as ::alloy_sol_types::SolStruct>::eip712_root_type()
            }
            fn eip712_encode_data(&self) -> ::alloy_sol_types::private::Vec<u8> {
                [
                    <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::SolType>::eip712_data_word(&self.xCoord)
                        .0,
                    <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::SolType>::eip712_data_word(&self.e)
                        .0,
                    <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::SolType>::eip712_data_word(&self.s)
                        .0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl ::alloy_sol_types::EventTopic for Signature {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.xCoord,
                    )
                    + <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::EventTopic>::topic_preimage_length(&rust.e)
                    + <::alloy_sol_types::sol_data::Uint<
                        256,
                    > as ::alloy_sol_types::EventTopic>::topic_preimage_length(&rust.s)
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut ::alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(
                    <Self as ::alloy_sol_types::EventTopic>::topic_preimage_length(rust),
                );
                <::alloy_sol_types::sol_data::Uint<
                    256,
                > as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.xCoord,
                    out,
                );
                <::alloy_sol_types::sol_data::Uint<
                    256,
                > as ::alloy_sol_types::EventTopic>::encode_topic_preimage(&rust.e, out);
                <::alloy_sol_types::sol_data::Uint<
                    256,
                > as ::alloy_sol_types::EventTopic>::encode_topic_preimage(&rust.s, out);
            }
            #[inline]
            fn encode_topic(
                rust: &Self::RustType,
            ) -> ::alloy_sol_types::abi::token::WordToken {
                let mut out = ::alloy_sol_types::private::Vec::new();
                <Self as ::alloy_sol_types::EventTopic>::encode_topic_preimage(
                    rust,
                    &mut out,
                );
                ::alloy_sol_types::abi::token::WordToken(
                    ::alloy_sol_types::private::keccak256(out),
                )
            }
        }
    };
    ///Module containing a contract's types and functions.
    /**

```solidity
interface IGateway {
    event GmpExecuted(bytes32 indexed id, bytes32 indexed source, address indexed dest, uint256 status, bytes32 result);
    event KeySetChanged(bytes32 indexed id, TssKey[] revoked, TssKey[] registered);
    function execute(Signature memory signature, GmpMessage memory message) external returns (uint8 status, bytes32 result);
    function updateKeys(Signature memory signature, UpdateKeysMessage memory message) external;
}
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::style)]
    pub mod IGateway {
        use super::*;
        /**Event with signature `GmpExecuted(bytes32,bytes32,address,uint256,bytes32)` and selector `0x29f318d7f9b869de8987630235eb1a1ea9d870133c3092af2510a9fed3e3da21`.
```solidity
event GmpExecuted(bytes32 indexed id, bytes32 indexed source, address indexed dest, uint256 status, bytes32 result);
```*/
        #[allow(non_camel_case_types, non_snake_case, clippy::style)]
        pub struct GmpExecuted {
            pub id: ::alloy_sol_types::private::FixedBytes<32>,
            pub source: ::alloy_sol_types::private::FixedBytes<32>,
            pub dest: ::alloy_sol_types::private::Address,
            pub status: ::alloy_sol_types::private::U256,
            pub result: ::alloy_sol_types::private::FixedBytes<32>,
        }
        #[allow(non_camel_case_types, non_snake_case, clippy::style)]
        const _: () = {
            #[automatically_derived]
            impl ::alloy_sol_types::SolEvent for GmpExecuted {
                type DataTuple<'a> = (
                    ::alloy_sol_types::sol_data::Uint<256>,
                    ::alloy_sol_types::sol_data::FixedBytes<32>,
                );
                type DataToken<'a> = <Self::DataTuple<
                    'a,
                > as ::alloy_sol_types::SolType>::Token<'a>;
                type TopicList = (
                    ::alloy_sol_types::sol_data::FixedBytes<32>,
                    ::alloy_sol_types::sol_data::FixedBytes<32>,
                    ::alloy_sol_types::sol_data::FixedBytes<32>,
                    ::alloy_sol_types::sol_data::Address,
                );
                const SIGNATURE: &'static str = "GmpExecuted(bytes32,bytes32,address,uint256,bytes32)";
                const SIGNATURE_HASH: ::alloy_sol_types::private::B256 = ::alloy_sol_types::private::B256::new([
                    41u8,
                    243u8,
                    24u8,
                    215u8,
                    249u8,
                    184u8,
                    105u8,
                    222u8,
                    137u8,
                    135u8,
                    99u8,
                    2u8,
                    53u8,
                    235u8,
                    26u8,
                    30u8,
                    169u8,
                    216u8,
                    112u8,
                    19u8,
                    60u8,
                    48u8,
                    146u8,
                    175u8,
                    37u8,
                    16u8,
                    169u8,
                    254u8,
                    211u8,
                    227u8,
                    218u8,
                    33u8,
                ]);
                const ANONYMOUS: bool = false;
                #[allow(unused_variables)]
                #[inline]
                fn new(
                    topics: <Self::TopicList as ::alloy_sol_types::SolType>::RustType,
                    data: <Self::DataTuple<'_> as ::alloy_sol_types::SolType>::RustType,
                ) -> Self {
                    Self {
                        id: topics.1,
                        source: topics.2,
                        dest: topics.3,
                        status: data.0,
                        result: data.1,
                    }
                }
                #[inline]
                fn tokenize_body(&self) -> Self::DataToken<'_> {
                    (
                        <::alloy_sol_types::sol_data::Uint<
                            256,
                        > as ::alloy_sol_types::SolType>::tokenize(&self.status),
                        <::alloy_sol_types::sol_data::FixedBytes<
                            32,
                        > as ::alloy_sol_types::SolType>::tokenize(&self.result),
                    )
                }
                #[inline]
                fn topics(
                    &self,
                ) -> <Self::TopicList as ::alloy_sol_types::SolType>::RustType {
                    (
                        Self::SIGNATURE_HASH.into(),
                        self.id.clone(),
                        self.source.clone(),
                        self.dest.clone(),
                    )
                }
                #[inline]
                fn encode_topics_raw(
                    &self,
                    out: &mut [::alloy_sol_types::abi::token::WordToken],
                ) -> ::alloy_sol_types::Result<()> {
                    if out.len()
                        < <Self::TopicList as ::alloy_sol_types::TopicList>::COUNT
                    {
                        return Err(::alloy_sol_types::Error::Overrun);
                    }
                    out[0usize] = ::alloy_sol_types::abi::token::WordToken(
                        Self::SIGNATURE_HASH,
                    );
                    out[1usize] = <::alloy_sol_types::sol_data::FixedBytes<
                        32,
                    > as ::alloy_sol_types::EventTopic>::encode_topic(&self.id);
                    out[2usize] = <::alloy_sol_types::sol_data::FixedBytes<
                        32,
                    > as ::alloy_sol_types::EventTopic>::encode_topic(&self.source);
                    out[3usize] = <::alloy_sol_types::sol_data::Address as ::alloy_sol_types::EventTopic>::encode_topic(
                        &self.dest,
                    );
                    Ok(())
                }
            }
        };
        /**Event with signature `KeySetChanged(bytes32,(uint8,uint256)[],(uint8,uint256)[])` and selector `0x08280779918718ff44abb0d69be5f2219690cd1c4155aa887ba7c2423a921e65`.
```solidity
event KeySetChanged(bytes32 indexed id, TssKey[] revoked, TssKey[] registered);
```*/
        #[allow(non_camel_case_types, non_snake_case, clippy::style)]
        pub struct KeySetChanged {
            pub id: ::alloy_sol_types::private::FixedBytes<32>,
            pub revoked: ::alloy_sol_types::private::Vec<
                <TssKey as ::alloy_sol_types::SolType>::RustType,
            >,
            pub registered: ::alloy_sol_types::private::Vec<
                <TssKey as ::alloy_sol_types::SolType>::RustType,
            >,
        }
        #[allow(non_camel_case_types, non_snake_case, clippy::style)]
        const _: () = {
            #[automatically_derived]
            impl ::alloy_sol_types::SolEvent for KeySetChanged {
                type DataTuple<'a> = (
                    ::alloy_sol_types::sol_data::Array<TssKey>,
                    ::alloy_sol_types::sol_data::Array<TssKey>,
                );
                type DataToken<'a> = <Self::DataTuple<
                    'a,
                > as ::alloy_sol_types::SolType>::Token<'a>;
                type TopicList = (
                    ::alloy_sol_types::sol_data::FixedBytes<32>,
                    ::alloy_sol_types::sol_data::FixedBytes<32>,
                );
                const SIGNATURE: &'static str = "KeySetChanged(bytes32,(uint8,uint256)[],(uint8,uint256)[])";
                const SIGNATURE_HASH: ::alloy_sol_types::private::B256 = ::alloy_sol_types::private::B256::new([
                    8u8,
                    40u8,
                    7u8,
                    121u8,
                    145u8,
                    135u8,
                    24u8,
                    255u8,
                    68u8,
                    171u8,
                    176u8,
                    214u8,
                    155u8,
                    229u8,
                    242u8,
                    33u8,
                    150u8,
                    144u8,
                    205u8,
                    28u8,
                    65u8,
                    85u8,
                    170u8,
                    136u8,
                    123u8,
                    167u8,
                    194u8,
                    66u8,
                    58u8,
                    146u8,
                    30u8,
                    101u8,
                ]);
                const ANONYMOUS: bool = false;
                #[allow(unused_variables)]
                #[inline]
                fn new(
                    topics: <Self::TopicList as ::alloy_sol_types::SolType>::RustType,
                    data: <Self::DataTuple<'_> as ::alloy_sol_types::SolType>::RustType,
                ) -> Self {
                    Self {
                        id: topics.1,
                        revoked: data.0,
                        registered: data.1,
                    }
                }
                #[inline]
                fn tokenize_body(&self) -> Self::DataToken<'_> {
                    (
                        <::alloy_sol_types::sol_data::Array<
                            TssKey,
                        > as ::alloy_sol_types::SolType>::tokenize(&self.revoked),
                        <::alloy_sol_types::sol_data::Array<
                            TssKey,
                        > as ::alloy_sol_types::SolType>::tokenize(&self.registered),
                    )
                }
                #[inline]
                fn topics(
                    &self,
                ) -> <Self::TopicList as ::alloy_sol_types::SolType>::RustType {
                    (Self::SIGNATURE_HASH.into(), self.id.clone())
                }
                #[inline]
                fn encode_topics_raw(
                    &self,
                    out: &mut [::alloy_sol_types::abi::token::WordToken],
                ) -> ::alloy_sol_types::Result<()> {
                    if out.len()
                        < <Self::TopicList as ::alloy_sol_types::TopicList>::COUNT
                    {
                        return Err(::alloy_sol_types::Error::Overrun);
                    }
                    out[0usize] = ::alloy_sol_types::abi::token::WordToken(
                        Self::SIGNATURE_HASH,
                    );
                    out[1usize] = <::alloy_sol_types::sol_data::FixedBytes<
                        32,
                    > as ::alloy_sol_types::EventTopic>::encode_topic(&self.id);
                    Ok(())
                }
            }
        };
        /**Function with signature `execute((uint256,uint256,uint256),(bytes32,uint128,address,uint128,uint256,uint256,bytes))` and selector `0x54b2b070`.
```solidity
function execute(Signature memory signature, GmpMessage memory message) external returns (uint8 status, bytes32 result);
```*/
        #[allow(non_camel_case_types, non_snake_case)]
        pub struct executeCall {
            pub signature: <Signature as ::alloy_sol_types::SolType>::RustType,
            pub message: <GmpMessage as ::alloy_sol_types::SolType>::RustType,
        }
        #[automatically_derived]
        #[allow(non_camel_case_types, non_snake_case)]
        impl ::core::clone::Clone for executeCall {
            #[inline]
            fn clone(&self) -> executeCall {
                executeCall {
                    signature: ::core::clone::Clone::clone(&self.signature),
                    message: ::core::clone::Clone::clone(&self.message),
                }
            }
        }
        ///Container type for the return parameters of the [`execute((uint256,uint256,uint256),(bytes32,uint128,address,uint128,uint256,uint256,bytes))`](executeCall) function.
        #[allow(non_camel_case_types, non_snake_case)]
        pub struct executeReturn {
            pub status: u8,
            pub result: ::alloy_sol_types::private::FixedBytes<32>,
        }
        #[automatically_derived]
        #[allow(non_camel_case_types, non_snake_case)]
        impl ::core::clone::Clone for executeReturn {
            #[inline]
            fn clone(&self) -> executeReturn {
                executeReturn {
                    status: ::core::clone::Clone::clone(&self.status),
                    result: ::core::clone::Clone::clone(&self.result),
                }
            }
        }
        #[allow(non_camel_case_types, non_snake_case, clippy::style)]
        const _: () = {
            {
                #[doc(hidden)]
                type UnderlyingSolTuple<'a> = (Signature, GmpMessage);
                #[doc(hidden)]
                type UnderlyingRustTuple<'a> = (
                    <Signature as ::alloy_sol_types::SolType>::RustType,
                    <GmpMessage as ::alloy_sol_types::SolType>::RustType,
                );
                #[automatically_derived]
                #[doc(hidden)]
                impl ::core::convert::From<executeCall> for UnderlyingRustTuple<'_> {
                    fn from(value: executeCall) -> Self {
                        (value.signature, value.message)
                    }
                }
                #[automatically_derived]
                #[doc(hidden)]
                impl ::core::convert::From<UnderlyingRustTuple<'_>> for executeCall {
                    fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                        Self {
                            signature: tuple.0,
                            message: tuple.1,
                        }
                    }
                }
            }
            {
                #[doc(hidden)]
                type UnderlyingSolTuple<'a> = (
                    ::alloy_sol_types::sol_data::Uint<8>,
                    ::alloy_sol_types::sol_data::FixedBytes<32>,
                );
                #[doc(hidden)]
                type UnderlyingRustTuple<'a> = (
                    u8,
                    ::alloy_sol_types::private::FixedBytes<32>,
                );
                #[automatically_derived]
                #[doc(hidden)]
                impl ::core::convert::From<executeReturn> for UnderlyingRustTuple<'_> {
                    fn from(value: executeReturn) -> Self {
                        (value.status, value.result)
                    }
                }
                #[automatically_derived]
                #[doc(hidden)]
                impl ::core::convert::From<UnderlyingRustTuple<'_>> for executeReturn {
                    fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                        Self {
                            status: tuple.0,
                            result: tuple.1,
                        }
                    }
                }
            }
            #[automatically_derived]
            impl ::alloy_sol_types::SolCall for executeCall {
                type Parameters<'a> = (Signature, GmpMessage);
                type Token<'a> = <Self::Parameters<
                    'a,
                > as ::alloy_sol_types::SolType>::Token<'a>;
                type Return = executeReturn;
                type ReturnTuple<'a> = (
                    ::alloy_sol_types::sol_data::Uint<8>,
                    ::alloy_sol_types::sol_data::FixedBytes<32>,
                );
                type ReturnToken<'a> = <Self::ReturnTuple<
                    'a,
                > as ::alloy_sol_types::SolType>::Token<'a>;
                const SIGNATURE: &'static str = "execute((uint256,uint256,uint256),(bytes32,uint128,address,uint128,uint256,uint256,bytes))";
                const SELECTOR: [u8; 4] = [84u8, 178u8, 176u8, 112u8];
                fn new<'a>(
                    tuple: <Self::Parameters<'a> as ::alloy_sol_types::SolType>::RustType,
                ) -> Self {
                    tuple.into()
                }
                fn tokenize(&self) -> Self::Token<'_> {
                    (
                        <Signature as ::alloy_sol_types::SolType>::tokenize(
                            &self.signature,
                        ),
                        <GmpMessage as ::alloy_sol_types::SolType>::tokenize(
                            &self.message,
                        ),
                    )
                }
                fn abi_decode_returns(
                    data: &[u8],
                    validate: bool,
                ) -> ::alloy_sol_types::Result<Self::Return> {
                    <Self::ReturnTuple<
                        '_,
                    > as ::alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                        .map(Into::into)
                }
            }
        };
        /**Function with signature `updateKeys((uint256,uint256,uint256),((uint8,uint256)[],(uint8,uint256)[]))` and selector `0x1498212f`.
```solidity
function updateKeys(Signature memory signature, UpdateKeysMessage memory message) external;
```*/
        #[allow(non_camel_case_types, non_snake_case)]
        pub struct updateKeysCall {
            pub signature: <Signature as ::alloy_sol_types::SolType>::RustType,
            pub message: <UpdateKeysMessage as ::alloy_sol_types::SolType>::RustType,
        }
        #[automatically_derived]
        #[allow(non_camel_case_types, non_snake_case)]
        impl ::core::clone::Clone for updateKeysCall {
            #[inline]
            fn clone(&self) -> updateKeysCall {
                updateKeysCall {
                    signature: ::core::clone::Clone::clone(&self.signature),
                    message: ::core::clone::Clone::clone(&self.message),
                }
            }
        }
        ///Container type for the return parameters of the [`updateKeys((uint256,uint256,uint256),((uint8,uint256)[],(uint8,uint256)[]))`](updateKeysCall) function.
        #[allow(non_camel_case_types, non_snake_case)]
        pub struct updateKeysReturn {}
        #[automatically_derived]
        #[allow(non_camel_case_types, non_snake_case)]
        impl ::core::clone::Clone for updateKeysReturn {
            #[inline]
            fn clone(&self) -> updateKeysReturn {
                updateKeysReturn {}
            }
        }
        #[allow(non_camel_case_types, non_snake_case, clippy::style)]
        const _: () = {
            {
                #[doc(hidden)]
                type UnderlyingSolTuple<'a> = (Signature, UpdateKeysMessage);
                #[doc(hidden)]
                type UnderlyingRustTuple<'a> = (
                    <Signature as ::alloy_sol_types::SolType>::RustType,
                    <UpdateKeysMessage as ::alloy_sol_types::SolType>::RustType,
                );
                #[automatically_derived]
                #[doc(hidden)]
                impl ::core::convert::From<updateKeysCall> for UnderlyingRustTuple<'_> {
                    fn from(value: updateKeysCall) -> Self {
                        (value.signature, value.message)
                    }
                }
                #[automatically_derived]
                #[doc(hidden)]
                impl ::core::convert::From<UnderlyingRustTuple<'_>> for updateKeysCall {
                    fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                        Self {
                            signature: tuple.0,
                            message: tuple.1,
                        }
                    }
                }
            }
            {
                #[doc(hidden)]
                type UnderlyingSolTuple<'a> = ();
                #[doc(hidden)]
                type UnderlyingRustTuple<'a> = ();
                #[automatically_derived]
                #[doc(hidden)]
                impl ::core::convert::From<updateKeysReturn>
                for UnderlyingRustTuple<'_> {
                    fn from(value: updateKeysReturn) -> Self {
                        ()
                    }
                }
                #[automatically_derived]
                #[doc(hidden)]
                impl ::core::convert::From<UnderlyingRustTuple<'_>>
                for updateKeysReturn {
                    fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                        Self {}
                    }
                }
            }
            #[automatically_derived]
            impl ::alloy_sol_types::SolCall for updateKeysCall {
                type Parameters<'a> = (Signature, UpdateKeysMessage);
                type Token<'a> = <Self::Parameters<
                    'a,
                > as ::alloy_sol_types::SolType>::Token<'a>;
                type Return = updateKeysReturn;
                type ReturnTuple<'a> = ();
                type ReturnToken<'a> = <Self::ReturnTuple<
                    'a,
                > as ::alloy_sol_types::SolType>::Token<'a>;
                const SIGNATURE: &'static str = "updateKeys((uint256,uint256,uint256),((uint8,uint256)[],(uint8,uint256)[]))";
                const SELECTOR: [u8; 4] = [20u8, 152u8, 33u8, 47u8];
                fn new<'a>(
                    tuple: <Self::Parameters<'a> as ::alloy_sol_types::SolType>::RustType,
                ) -> Self {
                    tuple.into()
                }
                fn tokenize(&self) -> Self::Token<'_> {
                    (
                        <Signature as ::alloy_sol_types::SolType>::tokenize(
                            &self.signature,
                        ),
                        <UpdateKeysMessage as ::alloy_sol_types::SolType>::tokenize(
                            &self.message,
                        ),
                    )
                }
                fn abi_decode_returns(
                    data: &[u8],
                    validate: bool,
                ) -> ::alloy_sol_types::Result<Self::Return> {
                    <Self::ReturnTuple<
                        '_,
                    > as ::alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                        .map(Into::into)
                }
            }
        };
        ///Container for all the [`IGateway`](self) function calls.
        pub enum IGatewayCalls {
            execute(executeCall),
            updateKeys(updateKeysCall),
        }
        #[automatically_derived]
        impl IGatewayCalls {
            /// All the selectors of this enum.
            ///
            /// Note that the selectors might not be in the same order as the
            /// variants, as they are sorted instead of ordered by definition.
            pub const SELECTORS: &'static [[u8; 4usize]] = &[
                [20u8, 152u8, 33u8, 47u8],
                [84u8, 178u8, 176u8, 112u8],
            ];
        }
        #[automatically_derived]
        impl ::alloy_sol_types::SolInterface for IGatewayCalls {
            const NAME: &'static str = "IGatewayCalls";
            const MIN_DATA_LENGTH: usize = 224usize;
            const COUNT: usize = 2usize;
            #[inline]
            fn selector(&self) -> [u8; 4] {
                match self {
                    Self::execute(_) => {
                        <executeCall as ::alloy_sol_types::SolCall>::SELECTOR
                    }
                    Self::updateKeys(_) => {
                        <updateKeysCall as ::alloy_sol_types::SolCall>::SELECTOR
                    }
                }
            }
            #[inline]
            fn selector_at(i: usize) -> ::core::option::Option<[u8; 4]> {
                Self::SELECTORS.get(i).copied()
            }
            #[inline]
            fn valid_selector(selector: [u8; 4]) -> bool {
                match selector {
                    <executeCall as ::alloy_sol_types::SolCall>::SELECTOR
                    | <updateKeysCall as ::alloy_sol_types::SolCall>::SELECTOR => true,
                    _ => false,
                }
            }
            #[inline]
            fn abi_decode_raw(
                selector: [u8; 4],
                data: &[u8],
                validate: bool,
            ) -> ::alloy_sol_types::Result<Self> {
                match selector {
                    <executeCall as ::alloy_sol_types::SolCall>::SELECTOR => {
                        <executeCall as ::alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(Self::execute)
                    }
                    <updateKeysCall as ::alloy_sol_types::SolCall>::SELECTOR => {
                        <updateKeysCall as ::alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(Self::updateKeys)
                    }
                    s => {
                        ::core::result::Result::Err(
                            ::alloy_sol_types::Error::unknown_selector(
                                <Self as ::alloy_sol_types::SolInterface>::NAME,
                                s,
                            ),
                        )
                    }
                }
            }
            #[inline]
            fn abi_encoded_size(&self) -> usize {
                match self {
                    Self::execute(inner) => {
                        <executeCall as ::alloy_sol_types::SolCall>::abi_encoded_size(
                            inner,
                        )
                    }
                    Self::updateKeys(inner) => {
                        <updateKeysCall as ::alloy_sol_types::SolCall>::abi_encoded_size(
                            inner,
                        )
                    }
                }
            }
            #[inline]
            fn abi_encode_raw(&self, out: &mut ::alloy_sol_types::private::Vec<u8>) {
                match self {
                    Self::execute(inner) => {
                        <executeCall as ::alloy_sol_types::SolCall>::abi_encode_raw(
                            inner,
                            out,
                        )
                    }
                    Self::updateKeys(inner) => {
                        <updateKeysCall as ::alloy_sol_types::SolCall>::abi_encode_raw(
                            inner,
                            out,
                        )
                    }
                }
            }
        }
        ///Container for all the [`IGateway`](self) events.
        pub enum IGatewayEvents {
            GmpExecuted(GmpExecuted),
            KeySetChanged(KeySetChanged),
        }
        #[automatically_derived]
        impl IGatewayEvents {
            /// All the selectors of this enum.
            ///
            /// Note that the selectors might not be in the same order as the
            /// variants, as they are sorted instead of ordered by definition.
            pub const SELECTORS: &'static [[u8; 32usize]] = &[
                [
                    8u8,
                    40u8,
                    7u8,
                    121u8,
                    145u8,
                    135u8,
                    24u8,
                    255u8,
                    68u8,
                    171u8,
                    176u8,
                    214u8,
                    155u8,
                    229u8,
                    242u8,
                    33u8,
                    150u8,
                    144u8,
                    205u8,
                    28u8,
                    65u8,
                    85u8,
                    170u8,
                    136u8,
                    123u8,
                    167u8,
                    194u8,
                    66u8,
                    58u8,
                    146u8,
                    30u8,
                    101u8,
                ],
                [
                    41u8,
                    243u8,
                    24u8,
                    215u8,
                    249u8,
                    184u8,
                    105u8,
                    222u8,
                    137u8,
                    135u8,
                    99u8,
                    2u8,
                    53u8,
                    235u8,
                    26u8,
                    30u8,
                    169u8,
                    216u8,
                    112u8,
                    19u8,
                    60u8,
                    48u8,
                    146u8,
                    175u8,
                    37u8,
                    16u8,
                    169u8,
                    254u8,
                    211u8,
                    227u8,
                    218u8,
                    33u8,
                ],
            ];
        }
        impl ::alloy_sol_types::SolEventInterface for IGatewayEvents {
            const NAME: &'static str = "IGatewayEvents";
            const COUNT: usize = 2usize;
            fn decode_log(
                topics: &[::alloy_sol_types::Word],
                data: &[u8],
                validate: bool,
            ) -> ::alloy_sol_types::Result<Self> {
                match topics.first().copied() {
                    Some(
                        <GmpExecuted as ::alloy_sol_types::SolEvent>::SIGNATURE_HASH,
                    ) => {
                        <GmpExecuted as ::alloy_sol_types::SolEvent>::decode_log(
                                topics,
                                data,
                                validate,
                            )
                            .map(Self::GmpExecuted)
                    }
                    Some(
                        <KeySetChanged as ::alloy_sol_types::SolEvent>::SIGNATURE_HASH,
                    ) => {
                        <KeySetChanged as ::alloy_sol_types::SolEvent>::decode_log(
                                topics,
                                data,
                                validate,
                            )
                            .map(Self::KeySetChanged)
                    }
                    _ => {
                        ::alloy_sol_types::private::Err(::alloy_sol_types::Error::InvalidLog {
                            name: <Self as ::alloy_sol_types::SolEventInterface>::NAME,
                            log: ::alloy_sol_types::private::Box::new(
                                ::alloy_sol_types::private::Log::new_unchecked(
                                    topics.to_vec(),
                                    data.to_vec().into(),
                                ),
                            ),
                        })
                    }
                }
            }
        }
    }
    impl From<[u8; 33]> for TssKey {
        fn from(bytes: [u8; 33]) -> Self {
            Self {
                yParity: if bytes[0] % 2 == 0 { 0 } else { 1 },
                xCoord: U256::from_be_slice(&bytes[1..]),
            }
        }
    }
    /// Extends the [`SolStruct`] to accept the chain_id and gateway contract address as parameter
    pub(crate) trait Eip712Ext {
        fn to_eip712_bytes_with_domain(
            &self,
            domain_separator: &Eip712Domain,
        ) -> Eip712Bytes;
        fn to_eip712_bytes(
            &self,
            chain_id: u64,
            gateway_contract: Address,
        ) -> Eip712Bytes {
            let domain_separator = eip712_domain_separator(chain_id, gateway_contract);
            Eip712Ext::to_eip712_bytes_with_domain(self, &domain_separator)
        }
        fn to_eip712_typed_hash(
            &self,
            chain_id: u64,
            gateway_contract: Address,
        ) -> Eip712Hash {
            let bytes = self.to_eip712_bytes(chain_id, gateway_contract);
            let mut hasher = Keccak256::new();
            hasher.update(bytes.as_ref());
            hasher.finalize().into()
        }
    }
    impl<T> Eip712Ext for T
    where
        T: SolStruct,
    {
        fn to_eip712_bytes_with_domain(
            &self,
            domain_separator: &Eip712Domain,
        ) -> Eip712Bytes {
            let mut digest_input = [0u8; 2 + 32 + 32];
            digest_input[0] = 0x19;
            digest_input[1] = 0x01;
            digest_input[2..34].copy_from_slice(&domain_separator.hash_struct()[..]);
            digest_input[34..66].copy_from_slice(&self.eip712_hash_struct()[..]);
            digest_input
        }
    }
    pub(crate) trait SignableMessage: Eip712Ext {
        type Method: SolCall;
        fn into_call(self, signature: Signature) -> Self::Method;
    }
    impl SignableMessage for GmpMessage {
        type Method = IGateway::executeCall;
        fn into_call(self, signature: Signature) -> Self::Method {
            IGateway::executeCall {
                signature,
                message: self,
            }
        }
    }
    impl SignableMessage for UpdateKeysMessage {
        type Method = IGateway::updateKeysCall;
        fn into_call(self, signature: Signature) -> Self::Method {
            IGateway::updateKeysCall {
                signature,
                message: self,
            }
        }
    }
}
