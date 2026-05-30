use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bdpm_ingest::normalize::{normalize_row, strip_salt, fields::strip_diacritics};
use bdpm_ingest::parse::ValidatedRow;
use bdpm_ingest::download::manifest::BDPMFile;

fn bench_strip_salt(c: &mut Criterion) {
    let inputs = vec![
        "chlorhydrate de paracétamol",
        "bésilate d'amlodipine",
        "paracetamol chlorhydrate monohydrate",
        "amoxicilline trihydratée",
        "arginine",
        "paracetamol 500mg",
        "diclofenac sodique",
        "MÉSILATE D'IMATINIB",
    ];

    c.bench_function("strip_salt", |b| {
        b.iter(|| {
            for input in &inputs {
                black_box(strip_salt(black_box(input)));
            }
        })
    });
}

fn bench_strip_diacritics(c: &mut Criterion) {
    let inputs = vec![
        "paracétamol",
        "bézilate d'amlodipine",
        "amoxicilline trihydratée",
        "méningocoque",
        "acétylsalicylique",
    ];

    c.bench_function("strip_diacritics", |b| {
        b.iter(|| {
            for input in &inputs {
                black_box(strip_diacritics(black_box(input)));
            }
        })
    });
}

fn bench_normalize_row_compo(c: &mut Criterion) {
    // Real CIS_COMPO rows (8 fields)
    let row = ValidatedRow {
        fields: vec![
            "64534169".into(),
            "Comprimes".into(),
            "307293".into(),
            "CHLORHYDRATE DE PARACETAMOL 500 mg".into(),
            "500".into(),
            "mg".into(),
            "5018".into(),
            "0".into(),
        ],
        line_number: 1,
    };

    c.bench_function("normalize_row CIS_COMPO", |b| {
        b.iter(|| {
            black_box(normalize_row(BDPMFile::CIS_COMPO_bdpm, black_box(&row)));
        })
    });
}

fn bench_normalize_row_drug(c: &mut Criterion) {
    // Real CIS_bdpm row (13 fields)
    let row = ValidatedRow {
        fields: vec![
            "60012483".into(),
            "Doliprane 1000 mg comprime".into(),
            "comprime".into(),
            "voie orale".into(),
            "autorise".into(),
            "nationale".into(),
            "comm".into(),
            "".into(),
            "EU/1/97/026/001".into(),
            "SANOFI AVENTIS".into(),
            "SANOFI".into(),
            "non".into(),
            "".into(),
        ],
        line_number: 1,
    };

    c.bench_function("normalize_row CIS_bdpm", |b| {
        b.iter(|| {
            black_box(normalize_row(BDPMFile::CIS_bdpm, black_box(&row)));
        })
    });
}

criterion_group!(
    benches,
    bench_strip_salt,
    bench_strip_diacritics,
    bench_normalize_row_compo,
    bench_normalize_row_drug,
);
criterion_main!(benches);