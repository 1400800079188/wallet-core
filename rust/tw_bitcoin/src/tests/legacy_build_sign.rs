#![allow(deprecated)]

use super::{hex, ONE_BTC};
use crate::modules::legacy::*;
use crate::modules::transactions::{BRC20TransferInscription, Brc20Ticker, OrdinalNftInscription};
use bitcoin::{PrivateKey, PublicKey, ScriptBuf};
use secp256k1::ffi::CPtr;
use secp256k1::XOnlyPublicKey;
use std::collections::HashMap;
use std::ffi::CString;
use tw_encoding::hex;
use tw_proto::Bitcoin::Proto as LegacyProto;
use tw_proto::Common::Proto as CommonProto;

#[test]
fn print_wif_keys() {
    let pk = PrivateKey::from_wif("cNt3XNHiJdJpoX5zt3CXY8ncgrCted8bxmFBzcGeTZbBw6jkByWB").unwrap();
    let seckey = tw_encoding::hex::encode(pk.to_bytes(), false);
    dbg!(seckey);

    let pubkey = pk.public_key(&secp256k1::Secp256k1::new());
    let pubkey = tw_encoding::hex::encode(pubkey.to_bytes(), false);
    dbg!(pubkey);
}

const FULL_SATOSHIS: i64 = (ONE_BTC * 50) as i64;
const SEND_SATOSHIS: i64 = FULL_SATOSHIS - (ONE_BTC / 100) as i64;

#[test]
fn ffi_proto_sign_input_p2pkh_output_p2pkh() {
    let alice_private_key = hex("56429688a1a6b00b90ccd22a0de0a376b6569d8684022ae92229a28478bfb657");
    let alice_pubkey = hex("036666dd712e05a487916384bfcd5973eb53e8038eccbbf97f7eed775b87389536");
    let bob_pubkey = hex("037ed9a436e11ec4947ac4b7823787e24ba73180f1edd2857bff19c9f4d62b65bf");

    let txid = hex("1e1cdc48aa990d7e154a161d5b5f1cad737742e97d2712ab188027bb42e6e47b")
        .into_iter()
        .rev()
        .collect();

    // Input.
    let input = unsafe {
        tw_build_p2pkh_script(FULL_SATOSHIS, alice_pubkey.as_c_ptr(), alice_pubkey.len()).into_vec()
    };
    let input: LegacyProto::TransactionOutput = tw_proto::deserialize(&input).unwrap();

    // Output.
    let output = unsafe {
        tw_build_p2pkh_script(SEND_SATOSHIS, bob_pubkey.as_c_ptr(), bob_pubkey.len()).into_vec()
    };
    let output: LegacyProto::TransactionOutput = tw_proto::deserialize(&output).unwrap();

    let signing = LegacyProto::SigningInput {
        private_key: vec![alice_private_key.into()],
        utxo: vec![LegacyProto::UnspentTransaction {
            out_point: Some(LegacyProto::OutPoint {
                hash: txid,
                index: 0,
                sequence: u32::MAX,
                ..Default::default()
            }),
            script: input.script,
            amount: input.value,
            variant: LegacyProto::TransactionVariant::P2PKH,
            spendingScript: Default::default(),
        }],
        plan: Some(LegacyProto::TransactionPlan {
            utxos: vec![LegacyProto::UnspentTransaction {
                out_point: Default::default(),
                script: output.script,
                amount: output.value,
                variant: LegacyProto::TransactionVariant::P2PKH,
                spendingScript: Default::default(),
            }],
            ..Default::default()
        }),
        ..Default::default()
    };

    let serialized = tw_proto::serialize(&signing).unwrap();

    let res = unsafe {
        tw_taproot_build_and_sign_transaction(serialized.as_c_ptr(), serialized.len()).into_vec()
    };

    let output: LegacyProto::SigningOutput = tw_proto::deserialize(&res).unwrap();
    dbg!(&output);
    assert_eq!(output.error, CommonProto::SigningError::OK);

    let encoded_hex = tw_encoding::hex::encode(output.encoded, false);

    assert_eq!(encoded_hex, "02000000017be4e642bb278018ab12277de9427773ad1c5f5b1d164a157e0d99aa48dc1c1e000000006a473044022078eda020d4b86fcb3af78ef919912e6d79b81164dbbb0b0b96da6ac58a2de4b102201a5fd8d48734d5a02371c4b5ee551a69dca3842edbf577d863cf8ae9fdbbd4590121036666dd712e05a487916384bfcd5973eb53e8038eccbbf97f7eed775b87389536ffffffff01c0aff629010000001976a9145eaaa4f458f9158f86afcba08dd7448d27045e3d88ac00000000");

    //todo!()
}

#[test]
fn ffi_proto_sign_input_p2pkh_output_p2wpkh() {
    let alice_private_key = hex("57a64865bce5d4855e99b1cce13327c46171434f2d72eeaf9da53ee075e7f90a");
    let alice_pubkey = hex("028d7dce6d72fb8f7af9566616c6436349c67ad379f2404dd66fe7085fe0fba28f");
    let bob_pubkey = hex("025a0af1510f0f24d40dd00d7c0e51605ca504bbc177c3e19b065f373a1efdd22f");

    let txid = hex("181c84965c9ea86a5fac32fdbd5f73a21a7a9e749fb6ab97e273af2329f6b911")
        .into_iter()
        .rev()
        .collect();

    // Input.
    let input = unsafe {
        tw_build_p2pkh_script(FULL_SATOSHIS, alice_pubkey.as_c_ptr(), alice_pubkey.len()).into_vec()
    };
    let input: LegacyProto::TransactionOutput = tw_proto::deserialize(&input).unwrap();

    // Output.
    let output = unsafe {
        tw_build_p2wpkh_script(SEND_SATOSHIS, bob_pubkey.as_c_ptr(), bob_pubkey.len()).into_vec()
    };
    let output: LegacyProto::TransactionOutput = tw_proto::deserialize(&output).unwrap();

    let signing = LegacyProto::SigningInput {
        private_key: vec![alice_private_key.into()],
        utxo: vec![LegacyProto::UnspentTransaction {
            out_point: Some(LegacyProto::OutPoint {
                hash: txid,
                index: 0,
                sequence: u32::MAX,
                ..Default::default()
            }),
            script: input.script,
            amount: input.value,
            variant: LegacyProto::TransactionVariant::P2PKH,
            spendingScript: Default::default(),
        }],
        plan: Some(LegacyProto::TransactionPlan {
            utxos: vec![LegacyProto::UnspentTransaction {
                out_point: Default::default(),
                script: output.script,
                amount: output.value,
                variant: LegacyProto::TransactionVariant::P2PKH,
                spendingScript: Default::default(),
            }],
            ..Default::default()
        }),
        ..Default::default()
    };

    let serialized = tw_proto::serialize(&signing).unwrap();

    let res = unsafe {
        tw_taproot_build_and_sign_transaction(serialized.as_c_ptr(), serialized.len()).into_vec()
    };

    let output: LegacyProto::SigningOutput = tw_proto::deserialize(&res).unwrap();
    dbg!(&output);
    assert_eq!(output.error, CommonProto::SigningError::OK);

    let encoded_hex = tw_encoding::hex::encode(output.encoded, false);

    assert_eq!(encoded_hex, "020000000111b9f62923af73e297abb69f749e7a1aa2735fbdfd32ac5f6aa89e5c96841c18000000006b483045022100df9ed0b662b759e68b89a42e7144cddf787782a7129d4df05642dd825930e6e6022051a08f577f11cc7390684bbad2951a6374072253ffcf2468d14035ed0d8cd6490121028d7dce6d72fb8f7af9566616c6436349c67ad379f2404dd66fe7085fe0fba28fffffffff01c0aff629010000001600140d0e1cec6c2babe8badde5e9b3dea667da90036d00000000");

    //todo!()
}

#[test]
fn ffi_proto_sign_input_p2pkh_output_p2tr_key_path() {
    let alice_private_key = hex("12ce558df23528f1aa86f1f51ac7e13a197a06bda27610fa89e13b04c40ee999");
    let alice_pubkey = hex("0351e003fdc48e7f31c9bc94996c91f6c3273b7ef4208a1686021bedf7673bb058");
    let bob_private_key = hex("26c2566adcc030a1799213bfd546e615f6ab06f72085ec6806ff1761da48d227");
    let bob_pubkey = hex("02c0938cf377023dfde55e9c96b3cff4ca8894fb6b5d2009006bd43c0bff69cac9");

    let txid = hex("c50563913e5a838f937c94232f5a8fc74e58b629fae41dfdffcc9a70f833b53a")
        .into_iter()
        .rev()
        .collect();

    // Input.
    let input = unsafe {
        tw_build_p2pkh_script(FULL_SATOSHIS, alice_pubkey.as_c_ptr(), alice_pubkey.len()).into_vec()
    };
    let input: LegacyProto::TransactionOutput = tw_proto::deserialize(&input).unwrap();

    // Output.
    let output = unsafe {
        tw_build_p2tr_key_path_script(SEND_SATOSHIS, bob_pubkey.as_c_ptr(), bob_pubkey.len()).into_vec()
    };
    let output: LegacyProto::TransactionOutput = tw_proto::deserialize(&output).unwrap();

    let signing = LegacyProto::SigningInput {
        private_key: vec![alice_private_key.into()],
        utxo: vec![LegacyProto::UnspentTransaction {
            out_point: Some(LegacyProto::OutPoint {
                hash: txid,
                index: 0,
                sequence: u32::MAX,
                ..Default::default()
            }),
            script: input.script,
            amount: input.value,
            variant: LegacyProto::TransactionVariant::P2PKH,
            spendingScript: Default::default(),
        }],
        plan: Some(LegacyProto::TransactionPlan {
            utxos: vec![LegacyProto::UnspentTransaction {
                out_point: Default::default(),
                script: output.script,
                amount: output.value,
                variant: LegacyProto::TransactionVariant::P2PKH,
                spendingScript: Default::default(),
            }],
            ..Default::default()
        }),
        ..Default::default()
    };

    let serialized = tw_proto::serialize(&signing).unwrap();

    let res = unsafe {
        tw_taproot_build_and_sign_transaction(serialized.as_c_ptr(), serialized.len()).into_vec()
    };

    let output: LegacyProto::SigningOutput = tw_proto::deserialize(&res).unwrap();
    dbg!(&output);
    assert_eq!(output.error, CommonProto::SigningError::OK);

    let encoded_hex = tw_encoding::hex::encode(output.encoded, false);

    assert_eq!(encoded_hex, "02000000013ab533f8709accfffd1de4fa29b6584ec78f5a2f23947c938f835a3e916305c5000000006b48304502210086ab2c2192e2738529d6cd9604d8ee75c5b09b0c2f4066a5c5fa3f87a26c0af602202afc7096aaa992235c43e712146057b5ed6a776d82b9129620bc5a21991c0a5301210351e003fdc48e7f31c9bc94996c91f6c3273b7ef4208a1686021bedf7673bb058ffffffff01c0aff62901000000225120e01cfdd05da8fa1d71f987373f3790d45dea9861acb0525c86656fe50f4397a600000000");

    //todo!()
}