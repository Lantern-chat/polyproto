// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::errors::ERR_MSG_DC_UID_MISMATCH;

use x509_cert::attr::AttributeTypeAndValue;

use super::*;

impl Constrained for Name {
    /// [Name] must meet the following criteria to be valid in the context of polyproto:
    /// - Distinguished name MUST have "common name" attribute, which is equal to the actor or
    ///   home server name of the subject in question. Only one "common name" is allowed.
    /// - MUST have AT LEAST one domain component, specifying the home server domain for this
    ///   entity.
    /// - If actor name, MUST include UID (OID 0.9.2342.19200300.100.1.1) and uniqueIdentifier
    ///   (OID 0.9.2342.19200300.100.1.44).
    ///     - UID is the federation ID of the actor.
    ///     - uniqueIdentifier is the [SessionId] of the actor.
    /// - MAY have "organizational unit" attributes
    /// - MAY have other attributes, which might be ignored by other home servers and other clients.
    // I apologize. This is horrible. I'll redo it eventually. Depression made me do it. -bitfl0wer
    fn validate(&self, target: Option<Target>) -> Result<(), ConstraintError> {
        log::trace!("[Name::validate()] Validating Name: {self}");
        let mut num_cn = 0;
        let mut num_dc = 0;
        let mut num_uid = 0;
        let mut num_unique_identifier = 0;
        let mut vec_dc: Vec<&RelativeDistinguishedName> = Vec::new();
        let mut uid: &RelativeDistinguishedName = &RelativeDistinguishedName::default();
        let mut cn: &RelativeDistinguishedName = &RelativeDistinguishedName::default();

        let rdns = &self.0;
        for rdn in rdns.iter() {
            log::trace!("[Name::validate()] Determining OID of RDN {rdn} and performing appropriate validation");

            for item in rdn.0.iter() {
                match item.oid {
                    oid if oid == OID_RDN_UID => {
                        log::trace!("[Name::validate()] Found UID in RDN: {item}");
                        num_uid += 1;
                        uid = rdn;
                        validate_rdn_uid(item)?;
                    }
                    oid if oid == OID_RDN_UNIQUE_IDENTIFIER => {
                        log::trace!("[Name::validate()] Found uniqueIdentifier in RDN: {item}");
                        num_unique_identifier += 1;
                        validate_rdn_unique_identifier(item)?;
                    }
                    oid if oid == OID_RDN_COMMON_NAME => {
                        log::trace!("[Name::validate()] Found Common Name in RDN: {item}");
                        num_cn += 1;
                        cn = rdn;
                        if num_cn > 1 {
                            return Err(ConstraintError::OutOfBounds {
                                lower: 1,
                                upper: 1,
                                actual: num_cn,
                                reason: "[Name::validate()] Distinguished Names must not contain more than one Common Name field".into()
                            });
                        }
                    }
                    oid if oid == OID_RDN_DOMAIN_COMPONENT => {
                        log::trace!("[Name::validate()] Found Domain Component in RDN: {item}");
                        num_dc += 1;
                        vec_dc.push(rdn);
                    }
                    _ => log::trace!(
                        "[Name::validate()] Found unknown/non-validated component in RDN: {item}"
                    ),
                }
            }
        }
        // The order of the DCs is reversed in the [Name] object, compared to the order of the DCs in the UID.
        vec_dc.reverse();
        if let Some(target) = target {
            match target {
                Target::Actor => {
                    log::trace!(
                        "[Name::validate()] Validating DC {vec_dc:?} matches DC in UID {uid}"
                    );

                    validate_dc_matches_dc_in_uid(&vec_dc, uid)?;
                }
                Target::HomeServer => {
                    if num_uid > 0 || num_unique_identifier > 0 {
                        return Err(ConstraintError::OutOfBounds {
                            lower: 0,
                            upper: 0,
                            actual: 1,
                            reason: "Home Servers must not have UID or uniqueIdentifier".into(),
                        });
                    }
                }
            };
        } else if num_uid != 0 {
            validate_dc_matches_dc_in_uid(&vec_dc, uid)?;
        }
        log::trace!(
            "Encountered {} UID components and {} Common Name components",
            num_uid,
            num_cn
        );
        if num_uid != 0 && num_cn != 0 {
            log::trace!("Validating UID username matches Common Name");
            validate_uid_username_matches_cn(uid, cn)?;
        }
        if num_dc == 0 {
            return Err(ConstraintError::OutOfBounds {
                lower: 1,
                upper: u8::MAX as usize,
                actual: 0,
                reason: "Domain Component is missing in Name component".into(),
            });
        }
        if num_uid > 1 {
            return Err(ConstraintError::OutOfBounds {
                lower: 0,
                upper: 1,
                actual: num_uid,
                reason: "Too many UID components supplied".into(),
            });
        }
        if num_unique_identifier > 1 {
            return Err(ConstraintError::OutOfBounds {
                lower: 0,
                upper: 1,
                actual: num_unique_identifier,
                reason: "Too many uniqueIdentifier components supplied".into(),
            });
        }
        if num_unique_identifier > 0 && num_uid == 0 {
            return Err(ConstraintError::OutOfBounds {
                lower: 1,
                upper: 1,
                actual: num_uid,
                reason: "Actors must have uniqueIdentifier AND UID, only uniqueIdentifier found"
                    .into(),
            });
        }
        if num_uid > 0 && num_unique_identifier == 0 {
            return Err(ConstraintError::OutOfBounds {
                lower: 1,
                upper: 1,
                actual: num_unique_identifier,
                reason: "Actors must have uniqueIdentifier AND UID, only UID found".into(),
            });
        }
        Ok(())
    }
}

/// Check if the domain components are equal between the UID and the DCs
fn validate_dc_matches_dc_in_uid(
    vec_dc: &[&RelativeDistinguishedName],
    uid: &RelativeDistinguishedName,
) -> Result<(), ConstraintError> {
    let uid_string = uid.to_string();

    // Find the position of the @ in the UID
    let position_of_at = match uid_string.find('@') {
        Some(pos) => pos,
        None => {
            log::warn!("[validate_dc_matches_dc_in_uid] UID {uid} does not contain an @",);
            return Err(ConstraintError::Malformed(Some(
                "UID does not contain an @".into(),
            )));
        }
    };
    // Split the UID at the @
    let uid_without_username = uid_string.split_at(position_of_at + 1).1; // +1 to not include the @

    let dc_normalized_uid: Vec<&str> = uid_without_username.split('.').collect();
    dbg!(&dc_normalized_uid);
    let mut index = 0u8;
    // Iterate over the DCs in the UID and check if they are equal to the DCs in the DCs
    for &component in dc_normalized_uid.iter() {
        let equivalent_dc = match vec_dc.get(index as usize) {
            Some(dc) => dc,
            None => {
                return Err(ConstraintError::Malformed(Some(
                    ERR_MSG_DC_UID_MISMATCH.into(),
                )));
            }
        };

        let equivalent_dc = equivalent_dc.to_string();
        if component != equivalent_dc.split_at(3).1 {
            return Err(ConstraintError::Malformed(Some(
                ERR_MSG_DC_UID_MISMATCH.into(),
            )));
        }
        index = match index.checked_add(1) {
            Some(i) => i,
            None => {
                return Err(ConstraintError::Malformed(Some(
                    "More than 255 Domain Components found".into(),
                )));
            }
        };
    }
    Ok(())
}

/// Validate the UID field in the RDN. This performs a regex check to see if the UID is a valid
/// Federation ID (FID).
fn validate_rdn_uid(item: &AttributeTypeAndValue) -> Result<(), ConstraintError> {
    let fid_regex = Regex::new(r"\b([a-z0-9._%+-]+)@([a-z0-9-]+(\.[a-z0-9-]+)*)")
        .expect("Regex failed to compile");

    if !fid_regex.is_match(&String::from_utf8_lossy(item.value.value())) {
        Err(ConstraintError::Malformed(Some(
            "Provided Federation ID (FID) in uid field seems to be invalid".into(),
        )))
    } else {
        Ok(())
    }
}

/// Validate the uniqueIdentifier field in the RDN. This performs a check to see if the provided
/// input is a valid [SessionId].
fn validate_rdn_unique_identifier(item: &AttributeTypeAndValue) -> Result<(), ConstraintError> {
    SessionId::new_validated(&String::from_utf8_lossy(item.value.value()))?;
    Ok(())
}

/// Validate that the UID username matches the Common Name
fn validate_uid_username_matches_cn(
    uid: &RelativeDistinguishedName,
    cn: &RelativeDistinguishedName,
) -> Result<(), ConstraintError> {
    // Find the position of the @ in the UID
    let uid_str = uid.to_string().split_off(4);
    let cn_str = cn.to_string().split_off(3);
    let position_of_at = match uid_str.find('@') {
        Some(pos) => pos,
        None => {
            log::warn!("[validate_dc_matches_dc_in_uid] UID \"{uid}\" does not contain an @",);
            return Err(ConstraintError::Malformed(Some(
                "UID does not contain an @".into(),
            )));
        }
    };
    // Split the UID at the @
    let uid_username_only = uid_str.split_at(position_of_at).0;
    match uid_username_only == cn_str {
        true => Ok(()),
        false => {
            log::warn!(
                "[validate_uid_username_matches_cn] UID username \"{}\" does not match the Common Name \"{}\"",
                uid_username_only,
                cn_str
            );
            Err(ConstraintError::Malformed(Some(
                "UID username does not match the Common Name".into(),
            )))
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::testing_utils::init_logger;

    use super::*;

    #[test]
    fn test_dc_matches_dc_in_uid() {
        let good_name = Name::from_str(
            "CN=flori,DC=polyphony,DC=chat,UID=flori@polyphony.chat,uniqueIdentifier=client1",
        )
        .unwrap();
        let bad_name = Name::from_str(
            "CN=flori,DC=polyphony,DC=chat,UID=flori@polyphonyy.chat,uniqueIdentifier=client1",
        )
        .unwrap();
        assert!(good_name.validate(Some(Target::Actor)).is_ok());
        assert!(bad_name.validate(Some(Target::Actor)).is_err());
        let bad_name = Name::from_str(
            "CN=flori,DC=polyphony,DC=chat,UID=flori@polyphony.cat,uniqueIdentifier=client1",
        )
        .unwrap();
        assert!(bad_name.validate(Some(Target::Actor)).is_err());
        assert!(bad_name.validate(Some(Target::Actor)).is_err());
        let bad_name = Name::from_str(
            "CN=flori,DC=polyphony,DC=chat,UID=flori@thisis.polyphony.chat,uniqueIdentifier=client1",
        )
        .unwrap();
        assert!(bad_name.validate(Some(Target::Actor)).is_err());
    }

    #[test]
    fn cn_has_to_match_uid_name() {
        init_logger();
        let cn = Name::from_str("cn=bitfl0wer").unwrap();
        let uid = Name::from_str("uid=flori@localhost").unwrap();
        assert!(
            validate_uid_username_matches_cn(uid.0.first().unwrap(), cn.0.first().unwrap())
                .is_err()
        );
        let cn = Name::from_str("cn=flori").unwrap();
        assert!(
            validate_uid_username_matches_cn(uid.0.first().unwrap(), cn.0.first().unwrap()).is_ok()
        );
        let good_name = Name::from_str(
            "CN=flori,DC=polyphony,DC=chat,UID=flori@polyphony.chat,uniqueIdentifier=client1",
        )
        .unwrap();
        let bad_name = Name::from_str(
            "CN=bitfl0wer,DC=polyphony,DC=chat,UID=flori@polyphony.chat,uniqueIdentifier=client1",
        )
        .unwrap();
        assert!(good_name.validate(None).is_ok());
        assert!(bad_name.validate(None).is_err());
        assert!(bad_name.validate(Some(Target::Actor)).is_err());
        assert!(bad_name.validate(Some(Target::HomeServer)).is_err());
        assert!(good_name.validate(Some(Target::Actor)).is_ok());
        assert!(good_name.validate(Some(Target::HomeServer)).is_err());
    }
}
