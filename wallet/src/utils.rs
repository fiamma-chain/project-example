use std::str::FromStr;

use bitcoin::{
    bip32::Xpriv, secp256k1, Address, KnownHrp, PrivateKey, PublicKey, ScriptBuf, XOnlyPublicKey,
};

use crate::provider::ProviderParams;

#[derive(Debug)]
pub struct Auxiliary {
    pub private_key: PrivateKey,
    pub pubkey: String,
    pub internal_x_only_pubkey: String,
    pub script_pk: ScriptBuf,
}

pub fn parse_private_key(private_key: &str, ctx: &ProviderParams) -> Auxiliary {
    let secp = secp256k1::Secp256k1::new();
    let private_key = if let Ok(pk) = PrivateKey::from_wif(private_key) {
        pk
    } else if let Ok(pk) = Xpriv::from_str(private_key) {
        pk.to_priv()
    } else {
        panic!("Invalid private key")
    };

    let pubkey = PublicKey::from_private_key(&secp, &private_key).to_string();
    let secret_key = private_key.inner;
    let keypair = secp256k1::Keypair::from_secret_key(&secp, &secret_key);
    let (internal_key, _parity) = XOnlyPublicKey::from_keypair(&keypair);
    let address = Address::p2tr(&secp, internal_key, None, KnownHrp::from(ctx.network));
    let internal_x_only_pubkey = internal_key.to_string();
    let script_pk = address.script_pubkey();
    Auxiliary {
        private_key,
        pubkey,
        internal_x_only_pubkey,
        script_pk,
    }
}
