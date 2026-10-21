use super::*;

#[test]
fn exposes_only_public_material_matching_the_active_signing_key() {
    let keys = AuthKeys::from_config(&STANDARD.encode([42; 32]), "runtime-key".to_owned()).unwrap();

    let public = keys.verification_key();
    let json = serde_json::to_value(&public).unwrap();

    assert_eq!(json.as_object().unwrap().len(), 2);
    assert_eq!(public.key_id, "runtime-key");
    assert_eq!(
        STANDARD.decode(public.public_key_base64).unwrap(),
        keys.signing_key.verifying_key().to_bytes()
    );
    assert_ne!(json["public_key_base64"], STANDARD.encode([42; 32]));
}
