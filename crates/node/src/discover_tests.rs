//! What `agentic discover lookup --file` takes from an address book
//! (spec/discovery-v1.md): a vCard, a CSV export or a plain list.
use super::*;

fn expected(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(k, h)| ((*k).to_owned(), (*h).to_owned()))
        .collect()
}

#[test]
fn an_address_book_yields_each_address_and_github_login_once_in_its_normal_form() {
    let vcard = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Ann Lee\r\n\
                 EMAIL;TYPE=INTERNET;TYPE=HOME:Ann.Lee@Gmail.com\r\n\
                 item1.EMAIL;TYPE=INTERNET:ann.lee+work@googlemail.com\r\n\
                 END:VCARD\r\nBEGIN:VCARD\r\nFN:Bob\r\nEMAIL:bob@example.org\r\nEND:VCARD\r\n";
    assert_eq!(
        handles_from(vcard),
        expected(&[
            ("google", "annlee@gmail.com"),
            ("google", "bob@example.org")
        ])
    );
    // Google Contacts puts several addresses of one person in one cell.
    let csv = "Name,E-mail 1 - Value,Notes\n\
               Carol,Carol@Example.org,\"likes rust, go\"\n\
               Dan,,\n\
               Eve,eve@example.org ::: eve.work@example.org,\n";
    assert_eq!(
        handles_from(csv),
        expected(&[
            ("google", "carol@example.org"),
            ("google", "eve@example.org"),
            ("google", "eve.work@example.org"),
        ])
    );
    // A parameter holding a dot is still a parameter; a link is not an
    // address, and a coin is not spent on it.
    assert_eq!(
        handles_from("item2.EMAIL;TYPE=a.b:Ann@Example.org\nsite,http://bob@example.org/about\n"),
        expected(&[("google", "ann@example.org")])
    );
    // Addresses pasted from a mail client: names beside them, several on a
    // line.
    assert_eq!(
        handles_from("Ann Lee <Ann.Lee@Gmail.com>; \"Bob\" <bob@example.org>, carol@example.org\n"),
        expected(&[
            ("google", "annlee@gmail.com"),
            ("google", "bob@example.org"),
            ("google", "carol@example.org"),
        ])
    );
    let list = "erin@example.org\ngithub:Octo-Cat\n  \nnot an address\ngithub:octo-cat\n";
    assert_eq!(
        handles_from(list),
        expected(&[("google", "erin@example.org"), ("github", "octo-cat")])
    );
}
