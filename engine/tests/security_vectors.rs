//! Independent published algorithm vectors, not paired encoder/decoder tests.
use hex_literal::hex;
use oxirush_security::common::{nea2_cipher_bits, nia2_mac};
#[test]
fn nia2_free5gc_byte_aligned_vector() {
    assert_eq!(
        nia2_mac(
            &hex!("d3c5d592327fb11c4035c6680af8c6d1"),
            0x398a59b4,
            0x1a,
            1,
            &hex!("484583d5afe082ae")
        ),
        0xb93787e6
    );
}
#[test]
fn nea2_ts33401_annex_c_test_set_1() {
    let mut data = hex!("981ba6824c1bfb1ab485472029b71d808ce33e2cc3c0b5fc1f3de8a6dc66b1f0");
    nea2_cipher_bits(
        &hex!("d3c5d592327fb11c4035c6680af8c6d1"),
        0x398a59b4,
        0x15,
        1,
        &mut data,
        253,
    );
    assert_eq!(
        data,
        hex!("e9fed8a63d155304d71df20bf3e82214b20ed7dad2f233dc3c22d7bdeeed8e78")
    );
}
#[test]
fn independent_python_5g_key_derivation_fixture() {
    use oxirush_security::nas_5gs::*;
    let v: serde_json::Value = serde_json::from_str(include_str!("kdf-vectors.json")).unwrap();
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let ck = hex!("b40ba9a3c58b2a05bbf0d987b21bf8cb");
    let ik = hex!("f769bcd751044604127672711c6d3441");
    let ausf = derive_kausf(&ck, &ik, ranterra_engine::auth::SN, &hex!("55f328b43577"));
    assert_eq!(hex(&ausf), v["kausf"]);
    let seaf = derive_kseaf(&ausf, ranterra_engine::auth::SN);
    assert_eq!(hex(&seaf), v["kseaf"]);
    let amf = derive_kamf(&seaf, "001010000000101", &[0, 0]);
    assert_eq!(hex(&amf), v["kamf"]);
    assert_eq!(hex(&derive_nas_key(&amf, 2, 2)[16..]), v["knas_int"]);
    assert_eq!(hex(&derive_nas_key(&amf, 1, 2)[16..]), v["knas_enc"]);
    assert_eq!(
        hex(&compute_xres_star(
            &ck,
            &ik,
            ranterra_engine::auth::SN,
            &hex!("23553cbe9637a89d218ae64dae47bf35"),
            &hex!("a54211d5e3ba50bf")
        )),
        v["res_star"]
    );
}
