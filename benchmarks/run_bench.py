#!/usr/bin/env python
import json
import os
import sys
import time


def bench_cracken_cmd(mask, minlen, maxlen):
    return './cracken -m {} -x {} {}'.format(minlen, maxlen, mask)


def bench_mp_cmd(mask, minlen, maxlen):
    return './mp64.bin -i {}:{} {}'.format(minlen, maxlen, mask)


def bench_crunch_cmd(mask, minlen, maxlen):
    # crunch uses different mask pattern
    mask = mask.replace('?d', '%').replace('?u', ',').replace('?l', '@')
    return './crunch {} {} -t {}'.format(minlen, maxlen, mask)


TOOLS = (
    ('cracken', bench_cracken_cmd),
    ('maskprocessor', bench_mp_cmd),
    ('crunch', bench_crunch_cmd),
)

BENCHES = (
    ('9digits', '?d?d?d?d?d?d?d?d?d', 9, 9),
    ('upper-5lower-digit', '?u?l?l?l?l?l?d', 7, 7),
    ('1-8digits', '?d?d?d?d?d?d?d?d', 1, 8),
)

MAX_BENCH_TIME = 120  # 2 minutes for each case benchmarks


def main():
    """simple script for running benchmarks of wordlist generation tools"""
    benchmarks = []

    for bench_name, mask, minlen, maxlen in BENCHES:
        for tool_name, fn in TOOLS:
            cmd = fn(mask, minlen, maxlen) + ' >/dev/null'
            print('\nrunning {!r}'.format(cmd))
            iters_took = 0
            iter = 0

            while iters_took < MAX_BENCH_TIME:
                print('.', end='')
                sys.stdout.flush()
                bench = {'tool': tool_name, 'bench': bench_name, 'iter': iter}
                benchmarks.append(bench)

                # run the bench
                took = -time.time()
                result = os.system(cmd)
                took += time.time()

                if result:
                    print('cmd failed')
                    bench['ok'] = False
                    break
                else:
                    bench.update({
                        'ok': True,
                        'took': took,
                    })
                iters_took += took

    with open('results.json', 'w') as fp:
        json.dump(benchmarks, fp, indent=4, sort_keys=True)


if __name__ == '__main__':
    main()
