use num_enum::TryFromPrimitive;

#[derive(TryFromPrimitive, Debug)]
#[repr(i32)]
pub enum CsgoPropType {
    DptInt = 0,
    DptFloat = 1,
    DptVector = 2,
    DptVectorXy = 3,
    DptString = 4,
    DptArray = 5,
    DptDataTable = 6,
    DptInt64 = 7,
}

pub const SPROP_UNSIGNED: i32 = 1 << 0;
pub const SPROP_COORD: i32 = 1 << 1;
pub const SPROP_NOSCALE: i32 = 1 << 2;
pub const SPROP_NORMAL: i32 = 1 << 5;
pub const SPROP_EXCLUDE: i32 = 1 << 6;
pub const SPROP_INSIDEARRAY: i32 = 1 << 8;
pub const SPROP_COLLAPSIBLE: i32 = 1 << 11;
pub const SPROP_COORD_MP: i32 =  1 << 12;
pub const SPROP_COORD_MP_LOWPRECISION: i32 =  1 << 13; 
pub const SPROP_COORD_MP_INTEGRAL: i32 =  1 << 14;
pub const SPROP_CELL_COORD: i32 =  1 << 15;
pub const SPROP_CELL_COORD_LOWPRECISION: i32 =  1 << 16;
pub const SPROP_CELL_COORD_INTEGRAL: i32 =  1 << 17;
pub const SPROP_CHANGES_OFTEN: i32 = 1 << 18;
pub const SPROP_VARINT: i32 = 1 << 19;