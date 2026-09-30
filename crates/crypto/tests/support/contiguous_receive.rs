use super::*;
use agentic_crypto::CryptoError;

#[test]
fn contiguous_receive_defers_129_future_originals_then_recovers_all_130_after_restart() {
    let (mut alice, mut bob) = pair();
    let mut wires = Vec::new();
    for n in 0..130 {
        let p = alice
            .encrypt(GROUP, format!("Original {n}: 📨").as_bytes(), AAD)
            .unwrap();
        wires.push(accept(&mut alice, p).wire);
    }
    let before = bob.snapshot().as_bytes().to_vec();
    for (n, wire) in wires.iter().enumerate().skip(1) {
        assert!(
            matches!(
                bob.decrypt_contiguous(GROUP, wire, AAD),
                Err(CryptoError::ReceiveGap)
            ),
            "future original {n} must remain pending"
        );
        assert_eq!(bob.snapshot().as_bytes(), before);
        if n == 64 {
            bob = MlsClient::restore(bob.snapshot().as_bytes()).unwrap();
        }
    }
    for (n, wire) in wires.iter().enumerate() {
        let before = bob.snapshot().as_bytes().to_vec();
        let p = bob.decrypt_contiguous(GROUP, wire, AAD).unwrap();
        assert_eq!(
            bob.snapshot().as_bytes(),
            before,
            "preparation is not a commit"
        );
        assert_eq!(p.value.plaintext, format!("Original {n}: 📨").as_bytes());
        assert_eq!(p.value.sender, b"alice-device");
        assert_eq!(p.value.epoch, 1);
        accept(&mut bob, p);
        if n == 64 {
            bob = MlsClient::restore(bob.snapshot().as_bytes()).unwrap();
        }
    }
    let before = bob.snapshot().as_bytes().to_vec();
    for n in [0, 64, 129] {
        assert!(matches!(
            bob.decrypt_contiguous(GROUP, &wires[n], AAD),
            Err(CryptoError::Mls)
        ));
    }
    assert_eq!(bob.snapshot().as_bytes(), before);
}

#[test]
fn contiguous_receive_only_reports_a_gap_for_a_valid_future_application() {
    let (mut alice, bob) = pair();
    let first = alice.encrypt(GROUP, b"First", AAD).unwrap();
    let first = accept(&mut alice, first).wire;
    let next = alice
        .encrypt(GROUP, b"Authenticated later application", AAD)
        .unwrap();
    let next = accept(&mut alice, next).wire;
    let mut corrupt = next.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    let mut trailing = next.clone();
    trailing.push(0);
    let before = bob.snapshot().as_bytes().to_vec();
    for bytes in [corrupt, trailing, next[..next.len() / 2].to_vec()] {
        assert!(matches!(
            bob.decrypt_contiguous(GROUP, &bytes, AAD),
            Err(CryptoError::Mls | CryptoError::InvalidInput)
        ));
        assert_eq!(bob.snapshot().as_bytes(), before);
    }
    assert!(matches!(
        bob.decrypt_contiguous(GROUP, &next, b"Wrong application context"),
        Err(CryptoError::ContextMismatch)
    ));
    assert!(matches!(
        bob.decrypt_contiguous([9; 32], &next, AAD),
        Err(CryptoError::UnknownGroup | CryptoError::ContextMismatch)
    ));
    assert!(matches!(
        bob.decrypt_contiguous(GROUP, &next, AAD),
        Err(CryptoError::ReceiveGap)
    ));
    assert_eq!(bob.snapshot().as_bytes(), before);
    assert_eq!(
        bob.decrypt_contiguous(GROUP, &first, AAD)
            .unwrap()
            .value
            .plaintext,
        b"First"
    );
}

#[test]
fn contiguous_receive_keeps_retained_old_keys_and_restores_ordinary_reordering_policy() {
    let (mut alice, mut bob) = pair();
    let mut wires = Vec::new();
    for n in 0..130 {
        let p = alice
            .encrypt(GROUP, format!("message {n}").as_bytes(), AAD)
            .unwrap();
        wires.push(accept(&mut alice, p).wire);
    }
    // Existing ordinary delivery has retained key 0 while consuming generation 1.
    let p = bob.decrypt(GROUP, &wires[1], AAD).unwrap();
    accept(&mut bob, p);
    let p = bob.decrypt_contiguous(GROUP, &wires[0], AAD).unwrap();
    assert_eq!(p.value.plaintext, b"message 0");
    accept(&mut bob, p);
    let p = bob.decrypt_contiguous(GROUP, &wires[2], AAD).unwrap();
    assert_eq!(p.value.plaintext, b"message 2");
    accept(&mut bob, p);
    bob = MlsClient::restore(bob.snapshot().as_bytes()).unwrap();
    // The temporary no-forward policy must not leak into the committed group.
    for n in [129, 3].into_iter().chain(4..129) {
        let p = bob.decrypt(GROUP, &wires[n], AAD).unwrap();
        assert_eq!(p.value.plaintext, format!("message {n}").as_bytes());
        accept(&mut bob, p);
    }
}
