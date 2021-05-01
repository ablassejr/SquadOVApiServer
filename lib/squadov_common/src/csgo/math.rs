use nom::number::streaming::{
    le_f32,
};

#[derive(Debug)]
pub struct CsgoVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

named!(pub parse_csgo_vector<CsgoVector>,
    complete!(do_parse!(
        x: le_f32 >>
        y: le_f32 >>
        z: le_f32 >>
        (CsgoVector{
            x: x,
            y: y,
            z: z,
        })
    ))
);

#[derive(Debug)]
pub struct CsgoQAngle {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

named!(pub parse_csgo_qangle<CsgoQAngle>,
    complete!(do_parse!(
        x: le_f32 >>
        y: le_f32 >>
        z: le_f32 >>
        (CsgoQAngle{
            x: x,
            y: y,
            z: z,
        })
    ))
);
