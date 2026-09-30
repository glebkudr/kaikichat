#!/usr/bin/env python3
"""Conditional finite quorum/selection model; never produces protocol authority."""
import json
import math
import sys
from decimal import Decimal, localcontext
from fractions import Fraction

MAX_INPUT = 16 * 1024
MAX_POPULATION = 2**32
MAX_MEMBERS = 128


def integer(value, lower, upper):
    if type(value) is not int or not lower <= value <= upper:
        raise ValueError('integer range')
    return value


def fields(value, expected):
    if type(value) is not dict or set(value) != set(expected):
        raise ValueError('fields')


def units(values):
    if type(values) is not list or len(values) > MAX_MEMBERS:
        raise ValueError('keys')
    return sum(integer(v, 1, MAX_POPULATION) for v in values)


def rational(value):
    return f'{value.numerator}/{value.denominator}'


def committee(value):
    fields(value, ['model', 'honestKeys', 'adversarialKeys', 'committeeSize', 'quorum', 'grindCandidates'])
    honest = units(value['honestKeys'])
    adversarial = units(value['adversarialKeys'])
    population = integer(honest + adversarial, 1, MAX_POPULATION)
    size = integer(value['committeeSize'], 1, min(MAX_MEMBERS, population))
    quorum = integer(value['quorum'], 1, size)
    candidates = value['grindCandidates']
    if candidates is not None:
        integer(candidates, 1, 1_000_000)
    denominator = math.comb(population, size)
    distribution = [
        Fraction(math.comb(adversarial, b) * math.comb(honest, size-b), denominator)
        if b <= adversarial and size-b <= honest else Fraction(0)
        for b in range(size + 1)
    ]
    # Only a structural quorum guarantee is modelled. Locking/view changes are separate.
    intersection = sum(distribution[max(0, 2*quorum-size):], Fraction(0))
    unavailable = sum(distribution[size-quorum+1:], Fraction(0))
    bound = None if candidates is None else rational(min(Fraction(1), candidates * intersection))
    # This intentionally unsafe alternative is an explanatory counterexample, not selection input.
    with localcontext() as context:
        context.prec = 28
        def sqrt_weight(keys):
            return sum((Decimal(v).sqrt() for v in sorted(keys)), Decimal(0))
        bad_weight = sqrt_weight(value['adversarialKeys'])
        unsafe_share = str(bad_weight / (sqrt_weight(value['honestKeys']) + bad_weight))
    return {
        'model': 'committee', 'status': 'conditional_model', 'populationUnits': population,
        'adversarialUnits': adversarial, 'committeeSize': size, 'quorum': quorum,
        'adversarialUnitShare': rational(Fraction(adversarial, population)),
        'unsafeSqrtPerKeyShare': unsafe_share,
        'distribution': [{'byzantineUnits': b, 'probability': rational(p)} for b, p in enumerate(distribution)],
        'intersectionFailureProbability': rational(intersection),
        'unavailableWithoutByzantineProbability': rational(unavailable),
        'grinding': {'candidateLimit': candidates, 'intersectionFailureUpperBound': bound},
        'assumptions': {'selection': 'uniform_without_replacement', 'adversary': 'fixed_unit_set',
            'honestVoting': 'one_value_per_decision_domain_epoch', 'operatorIndependence': 'not_established',
            'grindingLimit': 'unknown' if candidates is None else 'caller_assumed',
            'consensusProtocol': 'not_modelled'},
    }


def quorum_model(value):
    fields(value, ['model', 'members', 'byzantine', 'quorum', 'onlineHonest'])
    count = integer(value['members'], 1, MAX_MEMBERS)
    faulty = integer(value['byzantine'], 0, count)
    quorum = integer(value['quorum'], 1, count)
    online = integer(value['onlineHonest'], 0, count-faulty)
    safe = 2*quorum > count+faulty
    witness = None
    if not safe:
        left = list(range(quorum))
        right = list(range(count-quorum, count))
        intersection = set(left) & set(right)
        # Put every shared signer in the Byzantine set, then fill the assumed faulty count.
        evil = sorted(intersection) + [v for v in range(count) if v not in intersection]
        witness = {'leftMembers': left, 'rightMembers': right, 'byzantineMembers': sorted(evil[:faulty])}
    return {'model': 'quorum', 'status': 'conditional_model', 'members': count, 'byzantine': faulty,
        'quorum': quorum, 'onlineHonest': online, 'honestIntersectionGuaranteed': safe,
        'canFinalizeWithoutByzantine': quorum <= online, 'conflictWitness': witness}


def evaluate(value):
    if type(value) is not dict:
        raise ValueError('object required')
    if value.get('model') == 'committee':
        return committee(value)
    if value.get('model') == 'quorum':
        return quorum_model(value)
    raise ValueError('model')


def unique_fields(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError('duplicate field')
        value[key] = item
    return value


def reject_constant(_):
    raise ValueError('non-JSON constant')


def main():
    try:
        if len(sys.argv) != 1:
            raise ValueError('arguments')
        raw = sys.stdin.buffer.read(MAX_INPUT + 1)
        if len(raw) > MAX_INPUT:
            raise ValueError('size')
        request = json.loads(raw.decode('utf-8'), object_pairs_hook=unique_fields, parse_constant=reject_constant)
        result = evaluate(request)
    except (ValueError, RecursionError):
        print('risk input invalid', file=sys.stderr)
        return 2
    print(json.dumps(result, separators=(',', ':'), allow_nan=False))
    return 0


if __name__ == '__main__':
    sys.exit(main())
