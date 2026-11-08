use workspace_core::surface::*;

fn service(id: &str, x: f64, y: f64, width: f64, height: f64) -> ServiceBounds {
    ServiceBounds {
        service_id: id.into(),
        x,
        y,
        width,
        height,
    }
}
fn layout(
    services: Vec<ServiceBounds>,
    exclusions: Vec<LogicalRect>,
    capture: bool,
) -> SurfaceLayout {
    SurfaceLayout {
        revision: 1,
        services,
        exclusions,
        modal_input_capture: capture,
    }
}

#[test]
fn scales_bounds_and_makes_holes_view_relative() {
    let plan = plan(
        &layout(
            vec![service("a", 100.0, 50.0, 400.0, 300.0)],
            vec![LogicalRect {
                x: 80.0,
                y: 60.0,
                width: 60.0,
                height: 100.0,
            }],
            false,
        ),
        1.5,
    )
    .unwrap();
    assert_eq!(
        plan[0].bounds,
        PixelRect {
            x: 150,
            y: 75,
            width: 600,
            height: 450
        }
    );
    // Exclusion is 120..210 x 90..240 physical; clipped to the view's left/top edge at 150/75.
    assert_eq!(
        plan[0].holes,
        vec![PixelRect {
            x: 0,
            y: 15,
            width: 60,
            height: 150
        }]
    );
    assert!(!plan[0].captured);
}

#[test]
fn exclusions_round_outward_while_service_bounds_round_normally() {
    // Scale 1.5 keeps every product exact in binary. Exclusion edges in physical pixels:
    // left 15.75, top 18.75, right 32.25, bottom 32.25.
    let exclusion = LogicalRect {
        x: 10.5,
        y: 12.5,
        width: 11.0,
        height: 9.0,
    };
    let plan = plan(
        &layout(
            vec![
                service("wide", 0.0, 0.0, 100.0, 100.0),
                // Same rectangle as the exclusion: bounds round to 16..32 x 19..32.
                service("same", 10.5, 12.5, 11.0, 9.0),
            ],
            vec![exclusion],
            false,
        ),
        1.5,
    )
    .unwrap();
    // Outward: floor(15.75)=15, floor(18.75)=18, ceil(32.25)=33, ceil(32.25)=33.
    // Normal rounding would give 16, 19, 32, 32 and leave a native pixel on every side.
    assert_eq!(
        plan[0].holes,
        vec![PixelRect {
            x: 15,
            y: 18,
            width: 18,
            height: 15
        }]
    );
    // Service bounds keep normal rounding, and the outward hole covers the whole view.
    assert_eq!(
        plan[1].bounds,
        PixelRect {
            x: 16,
            y: 19,
            width: 16,
            height: 13
        }
    );
    assert_eq!(
        plan[1].holes,
        vec![PixelRect {
            x: 0,
            y: 0,
            width: 16,
            height: 13
        }]
    );
}

#[test]
fn exclusions_outside_a_view_produce_no_hole() {
    let plan = plan(
        &layout(
            vec![service("a", 0.0, 0.0, 100.0, 100.0)],
            vec![LogicalRect {
                x: 200.0,
                y: 0.0,
                width: 50.0,
                height: 50.0,
            }],
            false,
        ),
        1.0,
    )
    .unwrap();
    assert!(plan[0].holes.is_empty());
}

#[test]
fn modal_capture_marks_every_view() {
    let plan = plan(
        &layout(
            vec![
                service("a", 0.0, 0.0, 10.0, 10.0),
                service("b", 20.0, 0.0, 10.0, 10.0),
            ],
            vec![],
            true,
        ),
        2.0,
    )
    .unwrap();
    assert!(plan.iter().all(|slot| slot.captured));
}

#[test]
fn rejects_invalid_layouts_and_scales() {
    let ok = layout(vec![service("a", 0.0, 0.0, 10.0, 10.0)], vec![], false);
    assert!(plan(&ok, 0.0).is_err());
    assert!(plan(&ok, f64::NAN).is_err());
    assert!(plan(&ok, 9.0).is_err());
    for bad in [
        layout(vec![service("a", f64::NAN, 0.0, 10.0, 10.0)], vec![], false),
        layout(vec![service("a", -1.0, 0.0, 10.0, 10.0)], vec![], false),
        layout(vec![service("a", 0.0, 0.0, 0.0, 10.0)], vec![], false),
        layout(
            vec![
                service("a", 0.0, 0.0, 10.0, 10.0),
                service("a", 0.0, 0.0, 10.0, 10.0),
            ],
            vec![],
            false,
        ),
        layout(
            (0..6)
                .map(|i| service(&format!("s{i}"), 0.0, 0.0, 10.0, 10.0))
                .collect(),
            vec![],
            false,
        ),
        layout(
            vec![],
            vec![LogicalRect {
                x: 0.0,
                y: 0.0,
                width: -5.0,
                height: 5.0,
            }],
            false,
        ),
        layout(
            vec![],
            (0..17)
                .map(|_| LogicalRect {
                    x: 0.0,
                    y: 0.0,
                    width: 5.0,
                    height: 5.0,
                })
                .collect(),
            false,
        ),
    ] {
        assert!(plan_for(&bad).is_err());
    }
}

fn plan_for(layout: &SurfaceLayout) -> workspace_core::Result<Vec<SlotPlan>> {
    plan(layout, 1.0)
}

#[test]
fn stale_revisions_are_detected() {
    assert!(is_stale(5, 4));
    assert!(!is_stale(5, 5));
    assert!(!is_stale(5, 6));
}

#[test]
fn layout_json_rejects_unknown_fields() {
    let good = r#"{"revision":2,"services":[{"serviceId":"a","x":0,"y":0,"width":1,"height":1}],"exclusions":[],"modalInputCapture":true}"#;
    assert!(serde_json::from_str::<SurfaceLayout>(good).is_ok());
    let bad = r#"{"revision":2,"services":[],"extra":1}"#;
    assert!(serde_json::from_str::<SurfaceLayout>(bad).is_err());
}
