//! The rows the viewer's Info panel lists about a photo, and the ones no preview may: a place,
//! and the camera's serial numbers. The fixtures are made by `tests/fixtures/make_located.py`.

use crate::support;

use anyview_core::{FactGroup, FactLabel, Facts, Peek};
use anyview_image::{ExifFacts, RasterPeek, picture_facts};
use support::{budget, bytes, fixture, sniffed};

const SERIALS: [&str; 4] = ["SN-BODY", "SN-LENS", "123456", "654321"];

type Row = (FactGroup, FactLabel, String);

fn rows(facts: &Facts) -> Vec<Row> {
    facts
        .rows()
        .iter()
        .map(|row| (row.group, row.label, row.value.as_str().to_owned()))
        .collect()
}

fn expected(rows: &[(FactGroup, FactLabel, &str)]) -> Vec<Row> {
    rows.iter()
        .map(|(g, l, v)| (*g, *l, (*v).to_owned()))
        .collect()
}

#[test]
fn a_located_jpeg_lists_picture_camera_and_location_rows() {
    let file = bytes("located.jpg");
    let facts = picture_facts(&file, &sniffed(&file, "located.jpg"));
    assert_eq!(
        rows(&facts),
        expected(&[
            (FactGroup::Picture, FactLabel::Colour, "RGB, 8-bit"),
            (FactGroup::Camera, FactLabel::Camera, "TestCam Located One"),
            (FactGroup::Camera, FactLabel::Lens, "TestLens 35mm f/2"),
            (FactGroup::Camera, FactLabel::Exposure, "1/200 s · f/2.8"),
            (FactGroup::Camera, FactLabel::ExposureBias, "-0.7 EV"),
            (FactGroup::Camera, FactLabel::Iso, "400"),
            (FactGroup::Camera, FactLabel::FocalLength, "35 mm"),
            (FactGroup::Camera, FactLabel::Flash, "Auto, fired"),
            (FactGroup::Camera, FactLabel::Taken, "1 May 2024 at 12:30"),
            (FactGroup::Camera, FactLabel::Software, "TestSoft 2.1"),
            (
                FactGroup::Picture,
                FactLabel::Copyright,
                "(c) Test Photographer"
            ),
            (FactGroup::Picture, FactLabel::Resolution, "300 dpi"),
            (
                FactGroup::Location,
                FactLabel::Coordinates,
                "37.7749° N, 122.4194° W"
            ),
            (FactGroup::Location, FactLabel::Altitude, "12 m"),
        ])
    );
}

#[test]
fn other_containers_list_what_their_exif_and_header_say() {
    // file, what the viewer can decode (so a colour row exists), rows that must be listed
    let cases: [(&str, bool, &[(FactGroup, FactLabel, &str)]); 2] = [
        (
            "located.png",
            true,
            &[
                (FactGroup::Camera, FactLabel::Camera, "TestCam Located One"),
                (FactGroup::Picture, FactLabel::Resolution, "300 dpi"),
                (
                    FactGroup::Location,
                    FactLabel::Coordinates,
                    "37.7749° N, 122.4194° W",
                ),
            ],
        ),
        (
            // The viewer cannot decode a HEIC, yet its EXIF is listed.
            "located.heic",
            false,
            &[
                (FactGroup::Camera, FactLabel::Camera, "TestCam Located One"),
                (FactGroup::Camera, FactLabel::Flash, "Auto, fired"),
                (
                    FactGroup::Location,
                    FactLabel::Coordinates,
                    "37.7749° N, 122.4194° W",
                ),
            ],
        ),
    ];
    for (name, decodes, listed) in cases {
        let file = bytes(name);
        let got = rows(&picture_facts(&file, &sniffed(&file, name)));
        let colour = (
            FactGroup::Picture,
            FactLabel::Colour,
            "RGB, 8-bit".to_owned(),
        );
        if decodes {
            assert_eq!(got.first(), Some(&colour), "{name}");
        } else {
            assert!(
                got.iter().all(|(_, label, _)| *label != FactLabel::Colour),
                "{name}: nothing decoded, so no colour"
            );
        }
        for row in expected(listed) {
            assert!(got.contains(&row), "{name}: {row:?}");
        }
        assert!(!format!("{got:?}").contains("SN-"), "{name}");
    }
}

#[test]
fn a_picture_with_nothing_recorded_lists_only_its_colour() {
    let file = bytes("plain.jpg");
    let facts = picture_facts(&file, &sniffed(&file, "plain.jpg"));
    let labels: Vec<_> = facts.rows().iter().map(|row| row.label).collect();
    assert_eq!(labels, [FactLabel::Colour]);
    let facts = picture_facts(b"not an image", &sniffed(&bytes("plain.jpg"), "plain.jpg"));
    assert!(facts.rows().is_empty());
}

#[test]
fn no_serial_number_is_read_or_listed() {
    for name in ["located.jpg", "located.png"] {
        let file = bytes(name);
        let sniffed = sniffed(&file, name);
        let exif = format!("{:?}", ExifFacts::read(&file));
        let panel = format!("{:?}", picture_facts(&file, &sniffed));
        let (src, sniffed) = fixture(name);
        let peeked = RasterPeek::peek(&src, &sniffed, &budget(100)).unwrap();
        let peek = format!("{:?}", RasterPeek::facts(&peeked));
        for serial in SERIALS {
            for (what, text) in [("exif", &exif), ("panel", &panel), ("peek", &peek)] {
                assert!(!text.contains(serial), "{name}: {what} holds {serial}");
            }
        }
    }
}

#[test]
fn a_preview_lists_the_camera_but_never_the_place() {
    for name in ["located.jpg", "located.png"] {
        let (src, sniffed) = fixture(name);
        let peeked = RasterPeek::peek(&src, &sniffed, &budget(100)).unwrap();
        let facts = RasterPeek::facts(&peeked);
        assert!(
            facts
                .rows()
                .iter()
                .any(|row| row.label == FactLabel::Camera),
            "{name}: the camera is listed"
        );
        for row in facts.rows() {
            assert_ne!(row.group, FactGroup::Location, "{name}: {:?}", row.label);
            assert!(!row.value.as_str().contains('°'), "{name}: {:?}", row.label);
        }
        // The one place the EXIF type offers a preview, as opposed to the panel, says nothing of it.
        assert_eq!(
            ExifFacts::read(&bytes(name))
                .camera_facts()
                .rows()
                .iter()
                .filter(|r| r.group == FactGroup::Location)
                .count(),
            0
        );
    }
}
