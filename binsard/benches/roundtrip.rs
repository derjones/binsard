use binsard::{Decode, Encode, EncodeHelper, DecodeHelper};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[derive(Encode, Decode, Debug, PartialEq)]
struct SensorReading {
    #[binsard(bits = 4)]
    channel: u8,
    #[binsard(bits = 12)]
    value: u16,
    active: bool,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct CompactFlags {
    flag_a: bool,
    flag_b: bool,
    flag_c: bool,
    #[binsard(bits = 3)]
    priority: u8,
    #[binsard(bits = 5)]
    seq_num: u8,
    payload: Vec<u8>,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[binsard(bits = 2)]
enum SmallEnum {
    Off,
    Low,
    High,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[binsard(bits = 4)]
enum SensorMessage {
    Reset,
    Reading(SensorReading),
    Batch { count: u8, active: bool },
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct Packet {
    #[binsard(bits = 4)]
    version: u8,
    mode: SmallEnum,
    msg: SensorMessage,
    #[binsard(bits = 13)]
    tracker_version: Option<u16>,
}

fn bench_sensor_reading(c: &mut Criterion) {
    let sensor = SensorReading { channel: 9, value: 3000, active: true };
    let encoded = sensor.encode();

    c.bench_function("sensor_encode", |b| {
        b.iter(|| black_box(&sensor).encode());
    });
    c.bench_function("sensor_decode", |b| {
        b.iter(|| SensorReading::decode(black_box(&encoded)));
    });
}

fn bench_packet(c: &mut Criterion) {
    let pkt = Packet {
        version: 3,
        mode: SmallEnum::High,
        msg: SensorMessage::Reading(SensorReading {
            channel: 12,
            value: 2048,
            active: false,
        }),
        tracker_version: Some(8191),
    };
    let encoded = pkt.encode();

    c.bench_function("packet_encode", |b| {
        b.iter(|| black_box(&pkt).encode());
    });
    c.bench_function("packet_decode", |b| {
        b.iter(|| Packet::decode(black_box(&encoded)));
    });
}

fn bench_compact_flags(c: &mut Criterion) {
    let flags = CompactFlags {
        flag_a: true,
        flag_b: false,
        flag_c: true,
        priority: 7,
        seq_num: 31,
        payload: vec![0xAA; 64],
    };
    let encoded = flags.encode();

    c.bench_function("compact_flags_encode", |b| {
        b.iter(|| black_box(&flags).encode());
    });
    c.bench_function("compact_flags_decode", |b| {
        b.iter(|| CompactFlags::decode(black_box(&encoded)));
    });
}

fn bench_write_partly(c: &mut Criterion) {
    c.bench_function("write_partly_12bit_x100", |b| {
        b.iter(|| {
            let mut helper = EncodeHelper::default();
            for _ in 0..100 {
                helper.write_partly(black_box(3000u64), 12);
            }
            helper.finish()
        });
    });
}

fn bench_read_partly(c: &mut Criterion) {
    let mut helper = EncodeHelper::default();
    for _ in 0..100 {
        helper.write_partly(3000u64, 12);
    }
    let data = helper.finish();

    c.bench_function("read_partly_12bit_x100", |b| {
        b.iter(|| {
            let mut helper = DecodeHelper::default();
            for _ in 0..100 {
                let _: u64 = helper.read_partly(black_box(&data), 12).unwrap();
            }
        });
    });
}

criterion_group!(
    benches,
    bench_sensor_reading,
    bench_packet,
    bench_compact_flags,
    bench_write_partly,
    bench_read_partly,
);
criterion_main!(benches);
