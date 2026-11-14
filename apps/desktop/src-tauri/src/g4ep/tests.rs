use super::{ports::*, types::*};
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
fn profile() -> CodingProfile {
    CodingProfile::new(8, 4, 1024 * 1024).unwrap()
}
fn peer(id: u8, address: &str) -> Peer {
    Peer {
        node: NodeId([id; 32]),
        address: address.parse().unwrap(),
    }
}
fn manifest(bytes: u64) -> PreservationManifest {
    let coding = profile();
    let layout = coding.layout(bytes).unwrap();
    let fragments = (0..layout.stripes)
        .flat_map(|stripe| {
            (0..coding.total_shards()).map(move |shard| FragmentDescriptor {
                coordinate: FragmentCoordinate {
                    stripe,
                    shard: shard as u8,
                },
                bytes: coding.shard_bytes(),
                sha256: Sha256Digest([0; 32]),
            })
        })
        .collect();
    PreservationManifest {
        codec: CodecId::new("rs-gf256-draft").unwrap(),
        object: ObjectDescriptor {
            bytes,
            sha256: Sha256Digest([1; 32]),
        },
        coding,
        fragments,
    }
}
#[test]
fn profiles_bound_resources_and_reject_invalid_redundancy() {
    for (k, m, bytes) in [
        (0, 4, 1024 * 1024),
        (1, 1, 1024 * 1024),
        (8, 0, 1024 * 1024),
        (255, 1, 65536),
        (8, 4, 0),
        (8, 4, 65535),
        (8, 4, 4 * 1024 * 1024 + 1),
        (200, 20, 4 * 1024 * 1024),
    ] {
        assert!(CodingProfile::new(k, m, bytes).is_err());
    }
    let p = profile();
    assert_eq!(p.data_shards(), 8);
    assert_eq!(p.parity_shards(), 4);
    assert_eq!(p.stripe_buffer_bytes(), 12 * 1024 * 1024);
    assert!(matches!(p.layout(u64::MAX), Err(Error::SourceLimit)));
    assert!(
        CodingProfile::new(2, 1, 65536)
            .unwrap()
            .layout(MAX_SOURCE_BYTES)
            .is_err()
    );
}
#[test]
fn seven_hundred_mb_planning_is_bounded_arithmetic_not_a_transfer() {
    let l = profile().layout(700 * 1024 * 1024).unwrap();
    assert_eq!(l.stripes, 88);
    assert_eq!(l.fragments, 1056);
    assert_eq!(l.encoded_bytes, 1056 * 1024 * 1024);
    assert_eq!(l.padding_bytes, 4 * 1024 * 1024);
    assert_eq!(profile().layout(700_000_000).unwrap().stripes, 84);
    assert_eq!(profile().layout(0).unwrap().fragments, 0);
    assert_eq!(profile().layout(8 * 1024 * 1024).unwrap().padding_bytes, 0);
}
#[test]
fn structural_manifests_require_complete_distinct_coordinates_and_exact_shard_lengths() {
    assert!(ValidatedManifest::new(manifest(1)).is_ok());
    assert!(ValidatedManifest::new(manifest(0)).is_ok());
    let mut m = manifest(1);
    m.fragments.pop();
    assert!(ValidatedManifest::new(m).is_err());
    let mut m = manifest(1);
    m.fragments[1] = m.fragments[0].clone();
    assert!(ValidatedManifest::new(m).is_err());
    let mut m = manifest(1);
    m.fragments[0].coordinate.stripe = 1;
    assert!(ValidatedManifest::new(m).is_err());
    let mut m = manifest(1);
    m.fragments[0].coordinate.shard = 12;
    assert!(ValidatedManifest::new(m).is_err());
    let mut m = manifest(1);
    m.fragments[0].bytes -= 1;
    assert!(ValidatedManifest::new(m).is_err());
    assert!(CodecId::new("../untrusted-codec").is_err());
    assert_eq!(manifest(1).codec.as_str(), "rs-gf256-draft");
}
#[test]
fn private_enrollment_has_no_public_or_implicit_discovery() {
    let bind: SocketAddr = "100.64.0.1:42430".parse().unwrap();
    let enrolled = peer(1, "100.64.0.2:42430");
    let network = PrivateNetwork::new(bind, vec![enrolled]).unwrap();
    assert_eq!(network.bind(), bind);
    assert_eq!(network.peers(), &[enrolled]);
    assert!(network.require_peer(&enrolled).is_ok());
    assert!(network.require_peer(&peer(2, "100.64.0.2:42430")).is_err());
    assert!(PrivateNetwork::new(bind, vec![]).is_err());
    assert!(PrivateNetwork::new(bind, vec![enrolled, enrolled]).is_err());
    assert!(PrivateNetwork::new(bind, vec![peer(2, "8.8.8.8:42430")]).is_err());
    assert!(PrivateNetwork::new("0.0.0.0:42430".parse().unwrap(), vec![enrolled]).is_err());
    for address in [
        "127.0.0.1:42430",
        "192.168.1.2:42430",
        "10.1.2.3:42430",
        "[fd00::1]:42430",
    ] {
        assert!(PrivateNetwork::new(bind, vec![peer(2, address)]).is_ok());
    }
}
#[test]
fn deadlines_and_cancellation_are_explicit_without_sleeping() {
    let context = OperationContext {
        cancellation: CancellationToken::new(),
        deadline: Instant::now() + Duration::from_secs(30),
    };
    context.checkpoint().unwrap();
    context.cancellation.cancel();
    assert!(matches!(context.checkpoint(), Err(Error::Cancelled)));
    let expired = OperationContext {
        cancellation: CancellationToken::new(),
        deadline: Instant::now(),
    };
    assert!(matches!(expired.checkpoint(), Err(Error::Deadline)));
}
#[test]
fn all_seven_service_ports_can_be_used_as_thread_safe_trait_objects() {
    fn accepts_service<T: ?Sized + Send + Sync>() {}
    accepts_service::<dyn Encoder>();
    accepts_service::<dyn Decoder>();
    accepts_service::<dyn StorageNode>();
    accepts_service::<dyn PeerDiscovery>();
    accepts_service::<dyn FragmentTransfer>();
    accepts_service::<dyn IntegrityVerification>();
    accepts_service::<dyn RepairManager>();
    // No fake implementations: these assertions check only adapter usability, not a working codec.
}
