use crate::bom::Manufacturer;
use crate::circuit::{CircuitNode, PartDetails};
use crate::designator::{Designator, DesignatorKind};
use crate::epin::EdgeLocation;
use crate::epin::PinLocation;
use crate::epin::{EPin, PinKind};
use crate::glyph::{make_ic_body, make_label, TextJustification};
use crate::pin;
use crate::smd::SizeCode;
use crate::utils::pin_list;

pub fn make_lt3092_current_source(part_number: &str) -> CircuitNode {
    assert!(part_number.starts_with("LT3092"));
    CircuitNode::IntegratedCircuit(PartDetails {
        label: part_number.into(),
        manufacturer: Manufacturer {
            name: "Analog Devices".to_string(),
            part_number: part_number.into(),
        },
        description: "Programmable Current Source/Limiter".to_string(),
        comment: "".to_string(),
        hide_pin_designators: false,
        pins: pin_list(vec![
            pin!("SET", Input, -100, West),
            pin!("OUT_1", PowerSource, -100, East),
            pin!("IN", PowerSink, 200, West),
            pin!("OUT_2", PowerSource, 200, East),
        ]),
        outline: vec![
            make_ic_body(-400, -200, 400, 300),
            make_label(-400, 300, "U?", TextJustification::BottomLeft),
            make_label(-400, -200, part_number, TextJustification::TopLeft),
        ],
        suppliers: vec![],
        designator: Designator {
            kind: DesignatorKind::IntegratedCircuit,
            index: None,
        },
        size: SizeCode::SOT223,
    })
}
