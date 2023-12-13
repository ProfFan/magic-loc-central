use nalgebra::{DMatrix, Vector3};
use tracing::debug;

use crate::configuration;

fn least_squares_solution(points: &[Vector3<f64>], distances: &[f64]) -> Option<Vector3<f64>> {
    if points.len() != distances.len() || points.is_empty() {
        return None;
    }

    // Initialize the guess for the unknown point (e.g., to the origin)
    let mut guess = Vector3::new(0.0, 0.0, 0.0);

    // Define the maximum number of iterations and a tolerance for convergence
    let max_iterations = 45;
    let tolerance = 1e-3;

    for _ in 0..max_iterations {
        // Compute the Jacobian matrix and the residuals
        let mut jacobian = DMatrix::zeros(points.len(), 3);
        let mut residuals = DMatrix::zeros(points.len(), 1);

        for (i, (&point, &distance)) in points.iter().zip(distances).enumerate() {
            let diff = point - guess;
            let dist = diff.norm();

            if dist < 1e-6 {
                // Zeroing out the Jacobian matrix
                jacobian[(i, 0)] = diff.x / 1e-6;
                jacobian[(i, 1)] = diff.y / 1e-6;
                jacobian[(i, 2)] = diff.z / 1e-6;
            } else {
                // Fill in the Jacobian matrix
                jacobian[(i, 0)] = diff.x / dist;
                jacobian[(i, 1)] = diff.y / dist;
                jacobian[(i, 2)] = diff.z / dist;
            }

            // Compute the residual
            residuals[(i, 0)] = dist - distance;
        }

        // Update the guess using the Gauss-Newton method
        let delta = jacobian.clone().pseudo_inverse(1e-9).unwrap() * residuals.clone();
        guess += delta.fixed_rows::<3>(0);

        // Check for convergence
        if residuals.norm_squared() < tolerance {
            return Some(guess);
        }
    }

    Some(guess) // Return the best guess
}

/// Try localize a point with the given distances to the anchors
///
/// The function returns the estimated point and the error
pub fn localize_point(distances: &[f64]) -> Option<Vector3<f64>> {
    let mut points = Vec::new();
    let mut distances_valid = Vec::<f64>::new();

    for (i, &distance) in distances.iter().enumerate() {
        if !distance.is_normal() {
            continue;
        }

        let point = configuration::COORDINATES[i];
        points.push(Vector3::new(point.0, point.1, point.2));
        distances_valid.push(distance);
    }

    return least_squares_solution(&points, &distances_valid);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_least_squares_solution() {
        let points = [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(1.0, 1.0, 1.0),
        ];
        let distances = [
            3.0f64.sqrt(),
            2.0f64.sqrt(),
            2.0f64.sqrt(),
            2.0f64.sqrt(),
            0.0,
        ];

        let solution = least_squares_solution(&points, &distances).unwrap();

        assert!((solution - Vector3::new(1.0, 1.0, 1.0)).norm() < 1e-6);
    }
}
