use std::ops::{self, Div, Mul, Neg};

use glam::DVec3;

use eyre::eyre;

use gcd::Gcd;

static EPSILON: f64 = 0.0001;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3D {
    x: f64,
    y: f64,
    z: f64,
}

impl Default for Point3D {
    fn default() -> Self {
        Self {
            x: 0.,
            y: 0.,
            z: 0.,
        }
    }
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

impl Neg for Point3D {
    type Output = Point3D;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

static MAX_DISTANCE: f64 = 4294967296.;

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

    /// Returns true if one of the component has value exceeding [`MAX_DISTANCE`]
    pub fn is_too_big(&self) -> bool {
        self.x.abs() > MAX_DISTANCE || self.y.abs() > MAX_DISTANCE || self.z.abs() > MAX_DISTANCE
    }

    pub fn normalize(&self) -> Self {
        let delta = self.x.powf(2.) + self.y.powf(2.) + self.z.powf(2.);
        let delta = delta.sqrt();

        *self / delta
    }

    pub fn simplify(&self) -> Self {
        let gcd1 = (self.x.round().abs() as usize).gcd(self.y.round().abs() as usize);
        let gcd2 = gcd1.gcd(self.z.round().abs() as usize);

        *self / gcd2 as f64
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

impl From<Point3D> for DVec3 {
    fn from(value: Point3D) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

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

    pub fn solve_cramer(&self, r: [f64; 2]) -> eyre::Result<[f64; 2]> {
        let denominator = self.determinant();

        if denominator <= EPSILON && denominator >= -EPSILON {
            return Err(eyre!("Determinant is 0."));
        }

        let x_nom = Matrix2x2::from([r[0], self.b, r[1], self.d]).determinant();
        let y_nom = Matrix2x2::from([self.a, r[0], self.c, r[1]]).determinant();

        Ok([x_nom / denominator, y_nom / denominator])
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
    pub fn intersect_with_line(&self, rhs: Self) -> eyre::Result<Point3D> {
        // TODO do the same stupid shit i did with intersecting plane because this looks like it will have edge cases
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
            return Err(eyre!("Fail to test intersecting point of two lines."));
        }

        Ok(self.source + self.direction * x)
    }

    pub fn intersect_with_plane(&self, rhs: Plane3D) -> eyre::Result<Point3D> {
        rhs.intersect_with_line(*self)
    }
}

pub enum SideOfPoint {
    In,
    Out,
    On,
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

    pub fn intersect_with_plane(&self, rhs: Self) -> eyre::Result<Line3D> {
        let normal = self.normal().cross(rhs.normal());

        if normal.is_zero() {
            return Err(eyre!("Normal vector is zero."));
        }

        // TODO: i need to be a math major, maybe there is a way better than this
        // this only happens in the worst case where there is no intersection
        // or it is very straight

        // test x = 0
        let m = Matrix2x2::from([self.y, self.z, rhs.y, rhs.z]);
        if let Ok([start_y, start_z]) = m.solve_cramer([self.w, rhs.w]) {
            return Ok(Line3D {
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
        if let Ok([start_x, start_z]) = m.solve_cramer([self.w, rhs.w]) {
            return Ok(Line3D {
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
        if let Ok([start_x, start_y]) = m.solve_cramer([self.w, rhs.w]) {
            return Ok(Line3D {
                source: Point3D {
                    x: start_x,
                    y: start_y,
                    z: 0.,
                },
                direction: normal,
            });
        }

        Err(eyre!("No intersection between two planes."))
    }

    pub fn intersect_with_two_planes(&self, plane1: Self, plane2: Self) -> eyre::Result<Point3D> {
        let line = self.intersect_with_plane(plane1)?;

        line.intersect_with_plane(plane2)
    }

    pub fn intersect_with_line(&self, rhs: Line3D) -> eyre::Result<Point3D> {
        let t_part = self.normal().dot(rhs.direction);

        if t_part <= EPSILON && t_part >= -EPSILON {
            return Err(eyre!(
                "Cannot find intersection between a plane and a line."
            ));
        }

        let z_part = self.normal().dot(rhs.source);
        let t = (self.w - z_part) / t_part;

        Ok(rhs.source + rhs.direction * t)
    }

    pub fn simplify(&self) -> Self {
        let gcd1 = (self.x.round().abs() as usize).gcd(self.y.round().abs() as usize);
        let gcd2 = gcd1.gcd(self.z.round().abs() as usize);
        let gcd3 = gcd2.gcd(self.w.round().abs() as usize);

        Self {
            x: self.x / gcd3 as f64,
            y: self.y / gcd3 as f64,
            z: self.z / gcd3 as f64,
            w: self.w / gcd3 as f64,
        }
    }

    pub fn with_distance(&self, d: f64) -> Self {
        Self {
            x: self.x,
            y: self.y,
            z: self.z,
            w: d,
        }
    }

    pub fn side_of_point(&self, point: Point3D) -> SideOfPoint {
        let distance_check = self.normal().dot(point) - self.w;

        if distance_check < -EPSILON {
            SideOfPoint::Out
        } else if distance_check > EPSILON {
            SideOfPoint::In
        } else {
            SideOfPoint::On
        }
    }

    pub fn get_equation(&self) -> String {
        format!(
            "{}x {} {}y {} {}z = {}",
            self.x,
            if self.y.is_sign_negative() { "-" } else { "+" },
            self.y.abs(),
            if self.z.is_sign_negative() { "-" } else { "+" },
            self.z.abs(),
            self.w
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct Polygon3D(Vec<Point3D>);

impl Polygon3D {
    /// Will check for duplication before adding.
    pub fn add_vertex(&mut self, v: Point3D) -> &mut Self {
        if !self.0.contains(&v) {
            self.0.push(v);
        }

        self
    }

    pub fn vertices(&self) -> &Vec<Point3D> {
        &self.0
    }

    pub fn centroid(&self) -> eyre::Result<Point3D> {
        if self.0.is_empty() {
            return Err(eyre!("Polygon has no vertices."));
        }

        Ok(self.0.iter().fold(Point3D::default(), |acc, e| acc + *e) / self.0.len() as f64)
    }

    // Returns the normal from a plane created by first 3 points in the polygon.
    pub fn normal(&self) -> eyre::Result<Point3D> {
        if self.0.len() < 3 {
            return Err(eyre!("Polygon has less than 3 vertices to be a plane."));
        }

        Ok(Plane3D::from_three_points(self.0[0], self.0[1], self.0[2]).normal())
    }

    // https://github.com/pwitvoet/mess/blob/master/MESS/Mapping/Brush.cs#L38
    /// Returns an [`Polygon`] with vertices sorted clockwise.
    pub fn with_sorted_vertices(&self) -> eyre::Result<Self> {
        let centroid = self.centroid()?;

        // Since it is a face, now we interpret it as if we are on a 2D plane.
        let forward = self.0[0] - centroid;
        // Right thumb rule
        let right = forward.cross(self.normal()?);

        let mut what = self
            .0
            .iter()
            .map(|vertex| {
                let vector = *vertex - centroid;
                let x = vector.dot(right);
                let y = vector.dot(forward);

                y.atan2(x)
            })
            .zip(&self.0)
            .collect::<Vec<(f64, &Point3D)>>();

        what.sort_by(|(angle_a, _), (angle_b, _)| angle_a.total_cmp(angle_b));

        Ok(what
            .into_iter()
            .map(|(_, vertex)| vertex.to_owned())
            .collect::<Vec<Point3D>>()
            .into())
    }

    /// Fan triangulation from a list of sorted vertices
    pub fn triangulate(&self, reverse: bool) -> eyre::Result<Vec<Triangle3D>> {
        if self.0.len() < 3 {
            return Err(eyre!("Polygon has less than 3 vertices."));
        }

        let triangulation_count = self.0.len() - 3 + 1;
        let sorted_vertices = self.with_sorted_vertices()?;

        Ok((0..triangulation_count)
            .map(|cur_tri| {
                if reverse {
                    [
                        sorted_vertices.vertices()[0],
                        sorted_vertices.vertices()[cur_tri + 2],
                        sorted_vertices.vertices()[cur_tri + 1],
                    ]
                    .into()
                } else {
                    [
                        sorted_vertices.vertices()[0],
                        sorted_vertices.vertices()[cur_tri + 1],
                        sorted_vertices.vertices()[cur_tri + 2],
                    ]
                    .into()
                }
            })
            .collect())
    }

    pub fn to_plane3d(&self) -> Plane3D {
        self.clone().into()
    }
}

impl From<Vec<Point3D>> for Polygon3D {
    fn from(value: Vec<Point3D>) -> Self {
        Self(value)
    }
}

impl From<Polygon3D> for Plane3D {
    fn from(value: Polygon3D) -> Self {
        Plane3D::from_three_points(
            value.vertices()[0],
            value.vertices()[1],
            value.vertices()[2],
        )
    }
}

pub struct Triangle3D(Polygon3D);

impl Triangle3D {
    pub fn get_triangle(&self) -> &Vec<Point3D> {
        return self.0.vertices();
    }

    pub fn normal(&self) -> Point3D {
        self.0.normal().unwrap()
    }

    pub fn with_sorted_vertices(&self) -> Self {
        let what = self.0.with_sorted_vertices();
        let huh = what.unwrap();

        Self(huh)
    }
}

impl TryFrom<Polygon3D> for Triangle3D {
    type Error = &'static str;

    fn try_from(value: Polygon3D) -> Result<Self, Self::Error> {
        if value.vertices().len() != 3 {
            Err("Polygon does not have exactly 3 vertices.")
        } else {
            Ok(Triangle3D(value))
        }
    }
}

impl From<[Point3D; 3]> for Triangle3D {
    fn from(value: [Point3D; 3]) -> Self {
        Self(Polygon3D(value.to_vec()))
    }
}

#[derive(Debug, Default)]
pub struct ConvexPolytope(Vec<Polygon3D>);

impl ConvexPolytope {
    pub fn with_face_count(count: usize) -> Self {
        Self(vec![Polygon3D::default(); count])
    }

    pub fn add_polygon(&mut self, p: Polygon3D) -> &mut Self {
        self.0.push(p);
        self
    }

    pub fn polygons(&self) -> &Vec<Polygon3D> {
        &self.0
    }

    pub fn polygons_mut(&mut self) -> &mut Vec<Polygon3D> {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct Solid3D(Vec<Plane3D>);

impl Solid3D {
    pub fn contains_point(&self, point: Point3D) -> bool {
        self.0.iter().fold(true, |acc, e| {
            matches!(e.side_of_point(point), SideOfPoint::In | SideOfPoint::On) && acc
        })
    }

    pub fn face_count(&self) -> usize {
        self.0.len()
    }

    pub fn faces(&self) -> &Vec<Plane3D> {
        &self.0
    }
}

impl From<Vec<Plane3D>> for Solid3D {
    fn from(value: Vec<Plane3D>) -> Self {
        Self(value)
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
            l1.intersect_with_line(l2).unwrap(),
            Point3D::from([2., 3., 4.])
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

    #[test]
    fn triangulate_polygon() {
        let a: Polygon3D = vec![
            [1., 1., 0.].into(),
            [-1., -1., 0.].into(),
            [-1., 1., 0.].into(),
            [1., -1., 0.].into(),
        ]
        .into();

        let huh = a.triangulate(false).unwrap();

        assert_eq!(huh.len(), 2);
        assert_eq!(huh[0].get_triangle()[0], huh[1].get_triangle()[0]);
        assert_eq!(huh[0].get_triangle()[2], huh[1].get_triangle()[1]);
        assert_ne!(huh[0].get_triangle()[1], huh[1].get_triangle()[2]);
    }

    #[test]
    fn another_from_three_points() {
        let plane = Plane3D::from_three_points(
            [87.42562584220406, 61.81925288250034, 115.30958489062226].into(),
            [87.42562584220406, 62.52635966368689, 114.60247810943571].into(),
            [88.2916512459885, 61.46569949190706, 114.956031500029].into(),
        );

        println!("{}", plane.get_equation());
    }
}