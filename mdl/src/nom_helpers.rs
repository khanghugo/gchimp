use glam::Vec3;
use nom::{IResult as _IResult, Parser, combinator::map, multi::count, number::complete::le_f32};

pub type IResult<'a, T> = _IResult<&'a [u8], T>;

pub fn vec3(i: &[u8]) -> IResult<Vec3> {
    map(count(le_f32, 3), |res| Vec3::from_slice(res.as_slice())).parse(i)
}
