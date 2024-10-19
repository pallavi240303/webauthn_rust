// use base64urlsafedata::HumanBinaryData;
// use serde::{Serialize, Deserialize};
// use webauthn_rs::prelude::{CredentialID, PasskeyRegistration};
// use webauthn_rs_proto::{AuthenticatorAttachment, COSEAlgorithm, RequestRegistrationExtensions, UserVerificationPolicy};


// #[derive(Debug, Serialize, Deserialize)]
// pub struct SerializablePasskeyRegistration {
//     pub policy: UserVerificationPolicy,
//     pub exclude_credentials: Vec<CredentialID>,
//     pub challenge: HumanBinaryData,
//     pub credential_algorithms: Vec<COSEAlgorithm>,
//     pub require_resident_key: bool,
//     pub authenticator_attachment: Option<AuthenticatorAttachment>,
//     pub extensions: RequestRegistrationExtensions,
//     pub allow_synchronised_authenticators: bool,
// }

// // Function to convert PasskeyRegistration to SerializablePasskeyRegistration
// impl From<&PasskeyRegistration> for SerializablePasskeyRegistration {
//     fn from(registration: &PasskeyRegistration) -> Self {
//         let rs = &registration.rs; // Access the private field rs
//         Self {
//             policy: rs.policy.clone(),
//             exclude_credentials: rs.exclude_credentials.clone(),
//             challenge: rs.challenge.clone(),
//             credential_algorithms: rs.credential_algorithms.clone(),
//             require_resident_key: rs.require_resident_key,
//             authenticator_attachment: rs.authenticator_attachment.clone(),
//             extensions: rs.extensions.clone(),
//             allow_synchronised_authenticators: rs.allow_synchronised_authenticators,
//         }
//     }
// }
