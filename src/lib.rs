use signature::Signer;

pub struct IdCsr {
    pub pub_key: String,
    pub federation_id: FederationId,
    pub session_id: String,
    pub expiry: Option<u64>,
}

impl IdCsr {
    pub fn sign(&self, private_key: impl Signer<Signature>) -> IdCert {
        let id_cert_tbs = IdCertTBS {
            pub_key: self.pub_key.clone(),
            federation_id: self.federation_id.clone(),
            session_id: self.session_id.clone(),
            expiry: self.expiry.unwrap_or(0),
            serial: "".to_string(),
        };
        let signature = private_key.sign(&id_cert_tbs.to_bytes());
        IdCert {
            pub_key: id_cert_tbs.pub_key,
            federation_id: id_cert_tbs.federation_id,
            session_id: id_cert_tbs.session_id,
            expiry: self.expiry.unwrap_or(0),
            serial: "".to_string(),
            signature,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct FederationId {
    pub actor_name: String,
    pub domain: String,
    pub tld: String,
}

pub struct IdCert {
    pub pub_key: String,
    pub federation_id: FederationId,
    pub session_id: String,
    pub expiry: u64,
    pub serial: String,
    pub signature: Signature,
}

pub struct IdCertTBS {
    pub pub_key: String,
    pub federation_id: FederationId,
    pub session_id: String,
    pub expiry: u64,
    pub serial: String,
}

impl IdCertTBS {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.pub_key.as_bytes());
        bytes.extend_from_slice(self.federation_id.actor_name.as_bytes());
        bytes.extend_from_slice(self.federation_id.domain.as_bytes());
        bytes.extend_from_slice(self.federation_id.tld.as_bytes());
        bytes.extend_from_slice(self.session_id.as_bytes());
        bytes.extend_from_slice(&self.expiry.to_be_bytes());
        bytes.extend_from_slice(self.serial.as_bytes());
        bytes.to_vec()
    }
}

pub struct Signature {
    pub(crate) signature: String,
    pub algorithm: SignatureAlgorithm,
}

impl Signature {
    pub fn as_bytes(&self) -> &[u8] {
        self.signature.as_bytes()
    }

    pub fn as_str(&self) -> &str {
        &self.signature
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum SignatureAlgorithm {
    ECDSA_SECP256R1_SHA256,
    ECDSA_SECP384R1_SHA384,
    ECDSA_SECP521R1_SHA512,
    ED25519,
    ED448,
}
