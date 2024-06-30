use std::ops::{self, Div, Mul};

use glam::DVec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3D {
    x: f64,
    y: f64,
    z: f64,
}

impl ops::Add for Point3D {
    type Output = Point3D;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl ops::Sub for Point3D {
    type Output = Point3D;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Mul<f64> for Point3D {
    type Output = Point3D;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Div<f64> for Point3D {
    type Output = Point3D;

    fn div(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl Point3D {
    pub fn dot(&self, rhs: Self) -> f64 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(&self, rhs: Self) -> Self {
        Self {
            x: Matrix2x2::from([self.y, self.z, rhs.y, rhs.z]).determinant(),
            y: -Matrix2x2::from([self.x, self.z, rhs.x, rhs.z]).determinant(),
            z: Matrix2x2::from([self.x, self.y, rhs.x, rhs.y]).determinant(),
        }
    }

    pub fn is_zero(&self) -> bool {
        (self.x == 0.0 || self.x == -0.0)
            && (self.y == 0.0 || self.y == -0.0)
            && (self.z == 0.0 || self.z == -0.0)
    }

    pub fn as_array(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

impl From<[f64; 3]> for Point3D {
    fn from(value: [f64; 3]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
        }
    }
}

impl From<DVec3> for Point3D {
    fn from(value: DVec3) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

// impl Into<Point3D> for DVec3 {
//     fn into(self) -> Point3D {
//         Point3D { x: self.x, y: self.y, z: self.z }
//     }
// }

// | a b |
// | c d |
#[derive(Debug)]
pub struct Matrix2x2<T> {
    a: T,
    b: T,
    c: T,
    d: T,
}

impl Matrix2x2<f64> {
    pub fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    pub fn solve_cramer(&self, r: [f64; 2]) -> Option<[f64; 2]> {
        let denominator = self.determinant();

        if denominator == 0. || denominator == -0. {
            return None;
        }

        let x_nom = Matrix2x2::from([r[0], self.b, r[1], self.d]).determinant();
        let y_nom = Matrix2x2::from([self.a, r[0], self.c, r[1]]).determinant();

        return Some([x_nom / denominator, y_nom / denominator]);
    }
}

impl From<[[f64; 2]; 2]> for Matrix2x2<f64> {
    fn from(value: [[f64; 2]; 2]) -> Self {
        Self {
            a: value[0][0],
            b: value[0][1],
            c: value[1][0],
            d: value[1][1],
        }
    }
}

impl From<[f64; 4]> for Matrix2x2<f64> {
    fn from(value: [f64; 4]) -> Self {
        Self {
            a: value[0],
            b: value[1],
            c: value[2],
            d: value[3],
        }
    }
}

impl From<&[f64]> for Matrix2x2<f64> {
    fn from(value: &[f64]) -> Self {
        Self {
            a: value[0],
            b: value[1],
            c: value[2],
            d: value[3],
        }
    }
}

// | a b c |
// | d e f |
// | g h i |
pub struct Matrix3x3<T> {
    a: T,
    b: T,
    c: T,
    d: T,
    e: T,
    f: T,
    g: T,
    h: T,
    i: T,
}

impl From<[[f64; 3]; 3]> for Matrix3x3<f64> {
    fn from(value: [[f64; 3]; 3]) -> Self {
        Self {
            a: value[0][0],
            b: value[0][1],
            c: value[0][2],
            d: value[1][0],
            e: value[1][1],
            f: value[1][2],
            g: value[2][0],
            h: value[2][1],
            i: value[2][2],
        }
    }
}

impl Matrix3x3<f64> {
    pub fn determinant(&self) -> f64 {
        self.a
            * Matrix2x2 {
                a: self.e,
                b: self.f,
                c: self.h,
                d: self.i,
            }
            .determinant()
            - self.b
                * Matrix2x2 {
                    a: self.d,
                    b: self.f,
                    c: self.g,
                    d: self.i,
                }
                .determinant()
            + self.c
                * Matrix2x2 {
                    a: self.d,
                    b: self.e,
                    c: self.g,
                    d: self.h,
                }
                .determinant()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line3D {
    source: Point3D,
    direction: Point3D,
}

impl Line3D {
    pub fn intersect_with_line(&self, rhs: Self) -> Option<Point3D> {
        let m = Matrix2x2::from([
            self.direction.x,
            -rhs.direction.x,
            self.direction.y,
            -rhs.direction.y,
        ]);

        let [x, y] =
            m.solve_cramer([-self.source.x + rhs.source.x, -self.source.y + rhs.source.y])?;
        let r3 = [
            self.direction.z,
            -rhs.direction.z,
            -self.source.z + rhs.source.z,
        ];

        if r3[0] * x + r3[1] * y != r3[2] {
            return None;
        }

        Some(self.source + self.direction * x)
    }

    pub fn intersect_with_plane(&self, rhs: Plane3D) -> Option<Point3D> {
        rhs.intersect_with_line(self.clone())
    }
}

#[derive(Debug, Clone, Copy)]
/// Represented as an equation: i*x + j*y + k*z = w
pub struct Plane3D {
    x: f64,
    y: f64,
    z: f64,
    w: f64,
}

impl Plane3D {
    pub fn normal(&self) -> Point3D {
        Point3D::from([self.x, self.y, self.z])
    }

    pub fn distance(&self) -> f64 {
        self.w
    }

    pub fn from_three_points(p1: Point3D, p2: Point3D, p3: Point3D) -> Self {
        let l1 = p2 - p1;
        let l2 = p3 - p1;
        let normal = l1.cross(l2);
        let w = normal.dot(p1);

        Self {
            x: normal.x,
            y: normal.y,
            z: normal.z,
            w,
        }
    }

    pub fn intersect_with_plane(&self, rhs: Self) -> Option<Line3D> {
        let normal = self.normal().cross(rhs.normal());

        if normal.is_zero() {
            return None;
        }

        // TODO: i need to be a math major, maybe there is a way better than this
        // this only happens in the worst case where there is no intersection
        // or it is very straight

        // test x = 0
        let m = Matrix2x2::from([self.y, self.z, rhs.y, rhs.z]);
        if let Some([start_y, start_z]) = m.solve_cramer([self.w, rhs.w]) {
            return Some(Line3D {
                source: Point3D {
                    x: 0.,
                    y: start_y,
                    z: start_z,
                },
                direction: normal,
            });
        }

        // test y = 0
        let m = Matrix2x2::from([self.x, self.z, rhs.x, rhs.z]);
        if let Some([start_x, start_z]) = m.solve_cramer([self.w, rhs.w]) {
            return Some(Line3D {
                source: Point3D {
                    x: start_x,
                    y: 0.,
                    z: start_z,
                },
                direction: normal,
            });
        }

        // test z = 0
        let m = Matrix2x2::from([self.x, self.y, rhs.x, rhs.y]);
        if let Some([start_x, start_y]) = m.solve_cramer([self.w, rhs.w]) {
            return Some(Line3D {
                source: Point3D {
                    x: start_x,
                    y: start_y,
                    z: 0.,
                },
                direction: normal,
            });
        }

        None
    }

    pub fn intersect_with_two_planes(&self, plane1: Self, plane2: Self) -> Option<Point3D> {
        let line = self.intersect_with_plane(plane1)?;

        line.intersect_with_plane(plane2)
    }

    pub fn intersect_with_line(self, rhs: Line3D) -> Option<Point3D> {
        let t_part = self.normal().dot(rhs.direction);

        if t_part == 0. || t_part == -0. {
            return None;
        }

        let z_part = self.normal().dot(rhs.source);
        let t = (self.w - z_part) / t_part;

        Some(rhs.source + rhs.direction * t)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Polygon {
    vertices: Vec<Point3D>,
}

impl Polygon {
    pub fn add_vertex(&mut self, v: Point3D) -> &mut Self {
        self.vertices.push(v);
        self
    }

    pub fn vertices(&self) -> &Vec<Point3D> {
        &self.vertices
    }
}

#[derive(Debug, Default)]
pub struct ConvexPolytope {
    polygons: Vec<Polygon>,
}

impl ConvexPolytope {
    pub fn with_face_count(count: usize) -> Self {
        Self {
            polygons: vec![Polygon::default(); count],
        }
    }

    pub fn add_polygon(&mut self, p: Polygon) -> &mut Self {
        self.polygons.push(p);
        self
    }

    pub fn polygons(&self) -> &Vec<Polygon> {
        &self.polygons
    }

    pub fn polygons_mut(&mut self) -> &mut Vec<Polygon> {
        &mut self.polygons
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn matrix3x3_determinant() {
        let m = Matrix3x3::from([[1., 2., 3.], [1., 2., 3.], [1., 2., 3.]]);

        assert_eq!(m.determinant(), 0.)
    }

    #[test]
    fn matrix3x3_determinant2() {
        let m = Matrix3x3::from([[1., 2., 3.], [3., 2., 1.], [3., 2., 1.]]);

        assert_eq!(m.determinant(), 0.)
    }

    #[test]
    fn cross_prod() {
        let v1 = Point3D::from([2., 3., 4.]);
        let v2 = Point3D::from([-1., 1., 2.]);

        assert_eq!(v1.cross(v2), Point3D::from([2., -8., 5.]))
    }

    #[test]
    fn cross_bad() {
        let v1 = Point3D::from([2., 3., 4.]);
        let v2 = Point3D::from([2., 3., 4.]);

        assert!(v1.cross(v2).is_zero())
    }

    #[test]
    fn cross_bad2() {
        let v1 = Point3D::from([0., 0., 1.]);
        let v2 = Point3D::from([0., 0., -1.]);

        assert!(v1.cross(v2).is_zero())
    }

    #[test]
    fn two_planes_interection() {
        let p1 = Plane3D {
            x: 1.,
            y: 2.,
            z: 1.,
            w: 1.,
        };
        let p2 = Plane3D {
            x: 2.,
            y: 3.,
            z: -2.,
            w: -2.,
        };

        assert_eq!(
            p1.intersect_with_plane(p2).unwrap(),
            Line3D {
                source: Point3D::from([0., 0., 1.]),
                direction: Point3D::from([-7., 4., -1.])
            }
        );
    }

    #[test]
    fn two_lines_intersection() {
        let l1 = Line3D {
            source: Point3D::from([-2., -1., 0.]),
            direction: Point3D::from([1., 1., 1.]),
        };
        let l2 = Line3D {
            source: Point3D::from([8., -6., -11.]),
            direction: Point3D::from([-2., 3., 5.]),
        };

        assert_eq!(
            l1.intersect_with_line(l2),
            Some(Point3D::from([2., 3., 4.]))
        )
    }

    #[test]
    fn plane_intersects_line() {
        let p = Plane3D {
            x: 3.,
            y: -2.,
            z: 1.,
            w: 10.,
        };
        let l = Line3D {
            source: Point3D::from([2., 1., 0.]),
            direction: Point3D::from([-1., 1., 3.]),
        };

        assert_eq!(
            p.intersect_with_line(l).unwrap(),
            Point3D::from([5., -2., -9.])
        );
    }

    #[test]
    fn intersection_of_three_planes() {
        let plane1 = Plane3D::from_three_points(
            [-16., -64., -16.].into(),
            [-16., -63., -16.].into(),
            [-16., -64., -15.].into(),
        );
        let plane2 = Plane3D::from_three_points(
            [-64., -16., -16.].into(),
            [-64., -16., -15.].into(),
            [-63., -16., -16.].into(),
        );
        let plane3 = Plane3D::from_three_points(
            [-64., -64., -16.].into(),
            [-63., -64., -16.].into(),
            [-64., -63., -16.].into(),
        );

        assert_eq!(
            plane1.intersect_with_two_planes(plane2, plane3).unwrap(),
            Point3D::from([-16., -16., -16.])
        );
    }
}
