//! The normal form handles are found by (spec/discovery-v1.md, "Handles").
#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn a_handle_is_found_by_its_normal_form_only() {
    let cases = [
        (
            "google",
            " Ann.Lee+work@GoogleMail.com ",
            Some("annlee@gmail.com"),
        ),
        ("google", "a.n.n@gmail.com", Some("ann@gmail.com")),
        // Only Gmail ignores dots and a +suffix: elsewhere they are
        // different addresses of different people.
        ("google", "A.B+x@Example.org", Some("a.b+x@example.org")),
        ("google", "ab@example.org", Some("ab@example.org")),
        ("google", "no-at-sign", None),
        ("google", "two words@example.org", None),
        ("google", "+tag@gmail.com", None),
        ("google", "x@localhost", None),
        ("google", "http://u@host.com/x", None),
        ("google", "a:b@example.org", None),
        ("github", "Octo-Cat", Some("octo-cat")),
        ("github", "-octo", None),
        ("github", "octo_cat", None),
        ("github", &"a".repeat(40), None),
        ("github", "annlee@gmail.com", None),
        ("telegram", "ann", None),
    ];
    for (kind, handle, expected) in cases {
        assert_eq!(
            normalize_handle(kind, handle).as_deref(),
            expected,
            "{kind} {handle}"
        );
    }
    assert_eq!(
        handle_digest("google", "Ann.Lee@gmail.com").unwrap(),
        handle_digest("google", "annlee+x@googlemail.com").unwrap()
    );
    assert_ne!(
        handle_digest("google", "a.b@example.org").unwrap(),
        handle_digest("google", "ab@example.org").unwrap()
    );
}
