//! Picking which procedure a call means, out of the ones sharing its name.
//!
//! A name usually names one procedure. When it names several, the call site
//! decides between them by the shape and types of its arguments, which is what
//! VB.NET calls overload resolution.
//!
//! The rule is the one VB.NET uses: a candidate wins if it is at least as good
//! as every rival for every argument, and strictly better for at least one.
//! That is deliberately not "add up the scores": a candidate much better for
//! one argument and slightly worse for another does not win on arithmetic, it
//! is simply not comparable, and the call is ambiguous. Saying so is more use
//! to the author than guessing.
//!
//! Resolution is a frontend semantic rule. The transitional interpreter uses
//! this same implementation until it consumes resolved calls from typed IR.

use crate::frontend::type_model::TypeName;

/// What a call resolved to, as an index into the candidates given.
#[derive(Debug, PartialEq, Eq)]
pub enum Resolution {
    /// Exactly one candidate fits.
    Single(usize),
    /// No candidate accepts a call of this shape.
    NoMatch,
    /// Several fit equally well, and none is better than the rest.
    Ambiguous(Vec<usize>),
}

/// What overload resolution needs to know about one parameter.
#[derive(Debug, Clone)]
pub struct ParamShape {
    pub ty: TypeName,
    pub is_optional: bool,
    pub is_param_array: bool,
}

/// How well an argument fits a parameter, best first.
///
/// The declaration order *is* the ranking, and the variants are compared by it.
/// The two that carry a distance use it as a tie-break, which is what makes an
/// `Integer` argument choose a `Long` parameter over a `Double` one: both
/// widen, but one widens less. Derived ordering compares the variant first and
/// the distance only within a variant, which is exactly that rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Fit {
    /// The same type. Nothing to convert.
    Exact,
    /// A conversion that cannot lose anything, and how far it reaches:
    /// `Integer` into `Long` is nearer than `Integer` into `Double`.
    Widening(u8),
    /// One side is `Variant`, so the fit is only known when the call is made.
    Variant,
    /// A conversion that can lose something, and how far it reaches.
    Narrowing(u8),
}

/// Picks the procedure a call means.
///
/// `argument_types` carries `None` where the caller could not work out an
/// argument's type. An unknown argument fits everything equally, so it narrows
/// the field by arity alone rather than by guessing.
pub fn resolve(candidates: &[Vec<ParamShape>], argument_types: &[Option<TypeName>]) -> Resolution {
    // The overwhelmingly common case: one procedure with that name. Reporting
    // it as unmatched here would replace a precise "wrong argument type" from
    // the caller's own checking with a vague "nothing fits".
    if candidates.len() == 1 {
        return Resolution::Single(0);
    }

    let viable: Vec<(usize, Vec<Fit>)> = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, params)| Some((index, fits(params, argument_types)?)))
        .collect();

    match viable.as_slice() {
        [] => return Resolution::NoMatch,
        [(only, _)] => return Resolution::Single(*only),
        _ => {}
    }

    let best: Vec<usize> = viable
        .iter()
        .filter(|(_, fits)| !viable.iter().any(|(_, rival)| is_better(rival, fits)))
        .map(|(index, _)| *index)
        .collect();

    match best.as_slice() {
        [single] => Resolution::Single(*single),
        [] => Resolution::Ambiguous(viable.into_iter().map(|(index, _)| index).collect()),
        _ => Resolution::Ambiguous(best),
    }
}

/// Whether `left` beats `right`: no worse anywhere, and better somewhere.
fn is_better(left: &[Fit], right: &[Fit]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(a, b)| a <= b)
        && left.iter().zip(right).any(|(a, b)| a < b)
}

/// How each argument fits this candidate, or `None` if the call cannot be made.
fn fits(params: &[ParamShape], argument_types: &[Option<TypeName>]) -> Option<Vec<Fit>> {
    let takes_rest = params.last().is_some_and(|param| param.is_param_array);
    let required = params
        .iter()
        .filter(|param| !param.is_optional && !param.is_param_array)
        .count();

    if argument_types.len() < required {
        return None;
    }
    if !takes_rest && argument_types.len() > params.len() {
        return None;
    }

    let mut result = Vec::with_capacity(argument_types.len());
    for (index, argument) in argument_types.iter().enumerate() {
        // Everything past the last declared parameter goes into the ParamArray,
        // and is measured against its element type.
        let param = match params.get(index) {
            Some(param) => param,
            None => params.last()?,
        };
        result.push(fit_of(argument.as_ref(), param));
    }
    Some(result)
}

fn fit_of(argument: Option<&TypeName>, param: &ParamShape) -> Fit {
    let Some(argument) = argument else {
        return Fit::Variant;
    };
    let declared = match (&param.ty, param.is_param_array) {
        (TypeName::Array(element), true) => element.as_ref(),
        (ty, _) => ty,
    };

    if argument.same_type(declared) {
        return Fit::Exact;
    }
    if matches!(argument, TypeName::Variant) || matches!(declared, TypeName::Variant) {
        return Fit::Variant;
    }
    match (numeric_rank(argument), numeric_rank(declared)) {
        (Some(from), Some(to)) if from <= to => Fit::Widening(to - from),
        (Some(from), Some(to)) => Fit::Narrowing(from - to),
        // Anything else that converts at all, such as a class to its base or
        // a number to a string, ranks as the furthest narrowing rather than as
        // unrelated, so a call that could be made still resolves; it just
        // loses to any candidate that converts less.
        _ => Fit::Narrowing(u8::MAX),
    }
}

/// Where a type sits on the numeric ladder, if it is on it at all.
///
/// Moving up the ladder cannot lose anything; moving down can. The distance
/// between two rungs is how far a conversion reaches, which is what separates
/// two candidates that both widen.
fn numeric_rank(ty: &TypeName) -> Option<u8> {
    Some(match ty {
        TypeName::Byte => 0,
        TypeName::Int16 => 1,
        TypeName::Int32 => 2,
        TypeName::Int64 => 3,
        TypeName::Decimal => 4,
        TypeName::Single => 5,
        TypeName::Double => 6,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(ty: TypeName) -> ParamShape {
        ParamShape {
            ty,
            is_optional: false,
            is_param_array: false,
        }
    }

    #[test]
    fn a_lone_candidate_wins_without_looking_at_the_arguments() {
        let candidates = vec![vec![param(TypeName::String)]];
        assert_eq!(
            resolve(&candidates, &[Some(TypeName::Int32)]),
            Resolution::Single(0)
        );
    }

    #[test]
    fn arity_alone_can_decide() {
        let candidates = vec![
            vec![param(TypeName::Double)],
            vec![param(TypeName::Double), param(TypeName::Double)],
        ];
        assert_eq!(
            resolve(&candidates, &[Some(TypeName::Double)]),
            Resolution::Single(0)
        );
        assert_eq!(
            resolve(
                &candidates,
                &[Some(TypeName::Double), Some(TypeName::Double)]
            ),
            Resolution::Single(1)
        );
    }

    #[test]
    fn an_exact_type_beats_one_that_would_convert() {
        let candidates = vec![vec![param(TypeName::String)], vec![param(TypeName::Int32)]];
        assert_eq!(
            resolve(&candidates, &[Some(TypeName::String)]),
            Resolution::Single(0)
        );
        assert_eq!(
            resolve(&candidates, &[Some(TypeName::Int32)]),
            Resolution::Single(1)
        );
    }

    /// Both widen, so the one that widens less wins.
    #[test]
    fn the_nearer_widening_wins() {
        let candidates = vec![vec![param(TypeName::Double)], vec![param(TypeName::Int32)]];
        assert_eq!(
            resolve(&candidates, &[Some(TypeName::Int16)]),
            Resolution::Single(1)
        );
    }

    #[test]
    fn widening_beats_narrowing() {
        let candidates = vec![vec![param(TypeName::Double)], vec![param(TypeName::Byte)]];
        assert_eq!(
            resolve(&candidates, &[Some(TypeName::Int32)]),
            Resolution::Single(0)
        );
    }

    #[test]
    fn nothing_fitting_is_reported_rather_than_guessed() {
        let candidates = vec![vec![param(TypeName::Int32)], vec![param(TypeName::String)]];
        assert_eq!(resolve(&candidates, &[]), Resolution::NoMatch);
    }

    /// Better for one argument and worse for another is not "better overall".
    #[test]
    fn a_call_that_could_go_either_way_is_ambiguous() {
        let candidates = vec![
            vec![param(TypeName::Int32), param(TypeName::Double)],
            vec![param(TypeName::Double), param(TypeName::Int32)],
        ];
        assert_eq!(
            resolve(&candidates, &[Some(TypeName::Int32), Some(TypeName::Int32)]),
            Resolution::Ambiguous(vec![0, 1])
        );
    }

    #[test]
    fn an_optional_parameter_lets_a_shorter_call_through() {
        let candidates = vec![
            vec![
                param(TypeName::Int32),
                ParamShape {
                    ty: TypeName::Int32,
                    is_optional: true,
                    is_param_array: false,
                },
            ],
            vec![param(TypeName::String)],
        ];
        assert_eq!(
            resolve(&candidates, &[Some(TypeName::Int32)]),
            Resolution::Single(0)
        );
    }

    #[test]
    fn a_param_array_absorbs_any_number_of_trailing_arguments() {
        let candidates = vec![
            vec![param(TypeName::Int32)],
            vec![ParamShape {
                ty: TypeName::Array(Box::new(TypeName::Int32)),
                is_optional: false,
                is_param_array: true,
            }],
        ];
        assert_eq!(
            resolve(
                &candidates,
                &[
                    Some(TypeName::Int32),
                    Some(TypeName::Int32),
                    Some(TypeName::Int32)
                ]
            ),
            Resolution::Single(1)
        );
    }
}
