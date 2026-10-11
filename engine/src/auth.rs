//! Synthetic in-process UDM/AUSF and USIM. Never accept real credentials.
//! Deterministic public lab keys and RAND are deliberately unsuitable for a live network.
use milenage::Milenage;
use oxirush_security::nas_5gs::*;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

pub type AuthenticatedKeys = (Zeroizing<[u8; 16]>, Zeroizing<[u8; 32]>);
pub const SN: &[u8] = b"5G:mnc001.mcc001.3gppnetwork.org";
pub const ABBA: [u8; 2] = [0, 0];
pub fn supi(id: u32) -> String {
    format!("00101{id:010}")
}
pub fn lab_key(id: u32) -> [u8; 16] {
    let mut h = Sha256::new();
    h.update(b"RANterra public synthetic USIM v1");
    h.update(id.to_be_bytes());
    h.finalize()[..16].try_into().unwrap()
}
pub fn challenge(seed: u32, id: u32, sqn: u64) -> [u8; 16] {
    let mut h = Sha256::new();
    h.update(b"RANterra reproducible RAND v1");
    h.update(seed.to_be_bytes());
    h.update(id.to_be_bytes());
    h.update(sqn.to_be_bytes());
    h.finalize()[..16].try_into().unwrap()
}
const OP: [u8; 16] = [0x42; 16];
pub struct Vector {
    pub rand: [u8; 16],
    pub autn: [u8; 16],
    pub xres: Zeroizing<[u8; 16]>,
    pub kamf: Zeroizing<[u8; 32]>,
}
fn keys(id: u32, ck: &[u8; 16], ik: &[u8; 16], concealed_sqn: &[u8; 6]) -> Zeroizing<[u8; 32]> {
    let ausf = Zeroizing::new(derive_kausf(ck, ik, SN, concealed_sqn));
    let seaf = Zeroizing::new(derive_kseaf(&ausf, SN));
    Zeroizing::new(derive_kamf(&seaf, &supi(id), &ABBA))
}
pub fn vector(seed: u32, id: u32, sqn: u64) -> Vector {
    let rand = challenge(seed, id, sqn);
    let mut m = Milenage::new_with_op(lab_key(id), OP);
    let (res, ck, ik, ak) = m.f2345(&rand);
    let sqn: [u8; 6] = sqn.to_be_bytes()[2..].try_into().unwrap();
    let mac = m.f1(&rand, &sqn, &[0x80, 0]);
    let concealed = std::array::from_fn(|i| sqn[i] ^ ak[i]);
    let mut autn = [0; 16];
    autn[..6].copy_from_slice(&concealed);
    autn[6..8].copy_from_slice(&[0x80, 0]);
    autn[8..].copy_from_slice(&mac);
    Vector {
        rand,
        autn,
        xres: Zeroizing::new(
            compute_xres_star(&ck, &ik, SN, &rand, &res)
                .try_into()
                .unwrap(),
        ),
        kamf: keys(id, &ck, &ik, &concealed),
    }
}
pub const MAX_SQN: u64 = (1u64 << 48) - 1;
#[derive(Debug)]
pub enum AuthenticationFailure {
    Mac,
    Synchronization([u8; 14]),
    InvalidSqn,
}
impl std::fmt::Display for AuthenticationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Mac => "AUTN MAC failure (5GMM cause 20)",
            Self::Synchronization(_) => "SQN freshness failure (5GMM cause 21)",
            Self::InvalidSqn => "USIM SQN exceeds 48-bit range",
        })
    }
}
fn auts(m: &mut Milenage, rand: &[u8; 16], sqn: u64) -> [u8; 14] {
    let seq: [u8; 6] = sqn.to_be_bytes()[2..].try_into().unwrap();
    let ak = m.f5star(rand);
    let mac = m.f1star(rand, &seq, &[0, 0]);
    let mut out = [0; 14];
    for i in 0..6 {
        out[i] = seq[i] ^ ak[i];
    }
    out[6..].copy_from_slice(&mac);
    out
}
fn recover_sqn(m: &mut Milenage, rand: &[u8; 16], auts: &[u8]) -> Result<u64, String> {
    if auts.len() != 14 {
        return Err("Invalid AUTS length".into());
    }
    let ak = m.f5star(rand);
    let sqn: [u8; 6] = std::array::from_fn(|i| auts[i] ^ ak[i]);
    let mac = m.f1star(rand, &sqn, &[0, 0]);
    if !bool::from(mac.ct_eq(&auts[6..])) {
        return Err("AUTS MAC-S failure".into());
    }
    let mut n = [0; 8];
    n[2..].copy_from_slice(&sqn);
    Ok(u64::from_be_bytes(n))
}
/// UDM verifies AUTS against the outstanding RAND. Caller consumes that challenge.
pub fn resynchronize(
    id: u32,
    rand: &[u8; 16],
    auts: &[u8],
    issued_sqn: u64,
) -> Result<u64, String> {
    let n = recover_sqn(&mut Milenage::new_with_op(lab_key(id), OP), rand, auts)?;
    let next = n
        .max(issued_sqn)
        .checked_add(1)
        .filter(|s| *s <= MAX_SQN)
        .ok_or("48-bit SQN exhausted")?;
    Ok(next)
}
pub fn authenticate(
    id: u32,
    key: [u8; 16],
    rand: [u8; 16],
    autn: [u8; 16],
    last_sqn: &mut u64,
) -> Result<AuthenticatedKeys, String> {
    authenticate_or_sync(id, key, rand, autn, last_sqn).map_err(|e| e.to_string())
}
/// Independently compute UE keys only after validating AUTN, separation bit and SQN.
pub fn authenticate_or_sync(
    id: u32,
    key: [u8; 16],
    rand: [u8; 16],
    autn: [u8; 16],
    last_sqn: &mut u64,
) -> Result<AuthenticatedKeys, AuthenticationFailure> {
    if *last_sqn > MAX_SQN {
        return Err(AuthenticationFailure::InvalidSqn);
    }
    let mut m = Milenage::new_with_op(key, OP);
    let (res, ck, ik, ak) = m.f2345(&rand);
    let sqn: [u8; 6] = std::array::from_fn(|i| autn[i] ^ ak[i]);
    let mac = m.f1(&rand, &sqn, &autn[6..8].try_into().unwrap());
    if !bool::from(mac.ct_eq(&autn[8..])) || autn[6] & 0x80 == 0 {
        return Err(AuthenticationFailure::Mac);
    }
    let mut n = [0; 8];
    n[2..].copy_from_slice(&sqn);
    let n = u64::from_be_bytes(n);
    if n <= *last_sqn {
        return Err(AuthenticationFailure::Synchronization(auts(
            &mut m, &rand, *last_sqn,
        )));
    }
    *last_sqn = n;
    Ok((
        Zeroizing::new(
            compute_xres_star(&ck, &ik, SN, &rand, &res)
                .try_into()
                .unwrap(),
        ),
        keys(id, &ck, &ik, &autn[..6].try_into().unwrap()),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    #[test]
    fn published_milenage_ts_35208_set_1() {
        let mut m = Milenage::new_with_op(
            hex!("465b5ce8b199b49faa5f0a2ee238a6bc"),
            hex!("cdc202d5123e20f62b6d676ac72cb318"),
        );
        let rand = hex!("23553cbe9637a89d218ae64dae47bf35");
        assert_eq!(*m.opc(), hex!("cd63cb71954a9f4e48a5994e37a02baf"));
        assert_eq!(
            m.f2345(&rand),
            (
                hex!("a54211d5e3ba50bf"),
                hex!("b40ba9a3c58b2a05bbf0d987b21bf8cb"),
                hex!("f769bcd751044604127672711c6d3441"),
                hex!("aa689c648370")
            )
        );
        assert_eq!(
            m.f1(&rand, &hex!("ff9bb4d0b607"), &hex!("b9b9")),
            hex!("4a9ffac354dfafb3")
        );
    }
    #[test]
    fn published_resynchronization_vectors() {
        let mut m = Milenage::new_with_op(
            hex!("465b5ce8b199b49faa5f0a2ee238a6bc"),
            hex!("cdc202d5123e20f62b6d676ac72cb318"),
        );
        let rand = hex!("23553cbe9637a89d218ae64dae47bf35");
        assert_eq!(
            m.f1star(&rand, &hex!("ff9bb4d0b607"), &hex!("b9b9")),
            hex!("01cfaf9ec4e871e9")
        );
        assert_eq!(m.f5star(&rand), hex!("451e8beca43b"));
        // Independent free5GC util v1.0.7 TestValidateAUTS; AMF=0000.
        let mut m = Milenage::new_with_opc([0; 16], hex!("c8ffd2aa7a43c926bf2b2826205b9030"));
        let rand = hex!("01000000000000002e6f0eb33b7ffde7");
        let expected = hex!("797d7a19ca27f99f4363d3ca24be");
        assert_eq!(auts(&mut m, &rand, 0x00052c8c338e), expected);
        assert_eq!(
            recover_sqn(&mut m, &rand, &expected).unwrap(),
            0x00052c8c338e
        );
        let mut bad = expected;
        bad[13] ^= 1;
        assert!(recover_sqn(&mut m, &rand, &bad).is_err());
    }
    #[test]
    fn independent_usim_and_ausf_and_replay() {
        let v = vector(42, 101, 1);
        let mut sqn = 0;
        let (res, key) = authenticate(101, lab_key(101), v.rand, v.autn, &mut sqn).unwrap();
        assert_eq!(res, v.xres);
        assert_eq!(key, v.kamf);
        assert!(authenticate(101, lab_key(101), v.rand, v.autn, &mut sqn).is_err());
        assert!(authenticate(101, lab_key(102), v.rand, v.autn, &mut 0).is_err());
        let mut bad = v.autn;
        bad[15] ^= 1;
        assert!(authenticate(101, lab_key(101), v.rand, bad, &mut 0).is_err());
    }
}
