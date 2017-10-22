from __future__ import print_function

EXTENDED = 'Ext'
TRIGGERS = {
    'P': '"Push"',
    'S': '"Switch"',
    'W': '"WalkOver"',
    'G': '"Gun"',
}

SPEEDS = {
    '----': 0,
    'Slow': 8,
    'Normal': 16,
    'Fast': 32,
    'Turbo': 64,
    'Inst': 16384,
}

LOCKS = {
    'No': None,
    'Blue': 0,
    'Red': 1,
    'Yell': 2,
}


def to_special_type(column):
    return int(column)


def to_extended(column):
    return column == EXTENDED


def to_trigger_and_only_once(column):
    return TRIGGERS[column[0]], column[1] == '1'


def to_wait(column):
    if column == '--':
        return 0.0
    return float(column.rstrip('s'))


def split_chunk(chunk, n_columns):
    return (line.split(None, n_columns - 1) for line in chunk)


def to_bool(column):
    if column == 'Yes':
        return True
    elif column == 'No' or column == '--':
        return False
    else:
        assert False, column


def height(ref, off=None):
    string = '{ to = "%s"' % (ref,)
    if off is not None:
        return string + ', off = %d }' % (off,)
    else:
        return string + ' }'


def doors(chunk):
    open_door = height('LowestCeiling', -4)
    close_door = height('Floor')

    ceilings = {
        'Open, Wait, Then Close': (open_door, close_door),
        'Open and Stay Open': (open_door, None),
        'Close and Stay Closed': (close_door, None),
        'Close, Wait, Then Open': (close_door, open_door),
    }

    print()
    print()
    print('### Doors ###')
    print()
    for row in split_chunk(chunk, 8):
        special_type = to_special_type(row[0])
        extended = to_extended(row[1])
        trigger, only_once = to_trigger_and_only_once(row[2])
        lock = LOCKS[row[3]]
        speed = SPEEDS[row[4]]
        wait = to_wait(row[5])
        monsters = to_bool(row[6])

        first, second = ceilings[row[7]]
        print('[[linedef]]')
        print('  special_type =', special_type)
        print('  trigger =', trigger)
        if extended:
            print('  extended = true')
        if only_once:
            print('  only_once = true')
        if monsters:
            print('  monsters = true')
        if lock is not None:
            print('  lock =', lock)
        print('  [linedef.move]')
        if wait > 0.0:
            print('    wait =', wait)
        if speed > 0.0:
            print('    speed =', speed)
        if second is None:
            print('    ceiling = { first =', first, '}')
        else:
            print('    [linedef.move.ceiling]')
            print('      first =', first)
            print('      second =', second)
        print()


def floors(chunk):
    first_floors = {
        'Absolute 24': height('Floor', 24),
        'Absolute 512': height('Floor', 24),
        'Abs Shortest Lower Texture': None,
        'None': None,
        'Highest Neighbor Floor': height('HighestFloor'),
        'Highest Neighbor Floor + 8': height('HighestFloor', 8),
        'Lowest Neighbor Ceiling': height('LowestCeiling'),
        'Lowest Neighbor Ceiling - 8': height('LowestCeiling', - 8),
        'Lowest Neighbor Floor': height('LowestFloor'),
        'Next Neighbor Floor': height('NextFloor'),
    }

    print()
    print()
    print('### Floors ###')
    print()
    for row in split_chunk(chunk, 10):
        special_type = to_special_type(row[0])
        extended = to_extended(row[1])
        trigger, only_once = to_trigger_and_only_once(row[2])
        speed = SPEEDS[row[4]]
        monsters = to_bool(row[7])

        first_floor = first_floors[row[9]]
        if first_floor is None:
            continue

        print('[[linedef]]')
        print('  special_type =', special_type)
        print('  trigger =', trigger)
        if extended:
            print('  extended = true')
        if only_once:
            print('  only_once = true')
        if monsters:
            print('  monsters = true')
        print('  [linedef.move]')
        if speed > 0.0:
            print('    speed =', speed)
        print('    floor = { first =', first_floor, '}')
        print()


def ceilings(chunk):
    first_ceilings = {
        '8 Above Floor': height('Floor', 8),
        'Floor': height('Floor'),
        'Highest Neighbor Ceiling': height('HighestCeiling'),
        'Highest Neighbor Floor': height('HighestFloor'),
        'Lowest Neighbor Ceiling': height('LowestCeiling'),
    }

    print()
    print()
    print('### Ceilings ###')
    print()
    for row in split_chunk(chunk, 10):
        special_type = to_special_type(row[0])
        extended = to_extended(row[1])
        trigger, only_once = to_trigger_and_only_once(row[2])
        speed = SPEEDS[row[4]]
        monsters = to_bool(row[7])
        first_ceiling = first_ceilings[row[9]]

        print('[[linedef]]')
        print('  special_type =', special_type)
        print('  trigger =', trigger)
        if extended:
            print('  extended = true')
        if only_once:
            print('  only_once = true')
        if monsters:
            print('  monsters = true')
        print('  [linedef.move]')
        if speed > 0.0:
            print('    speed =', speed)
        print('    ceiling = { first =', first_ceiling, '}')
        print()


def platforms(chunk):
    floors = {
        'Ceiling (toggle)': None,
        'Lowest and Highest Floor (perpetual)': (height('LowestFloor'),
                                                 height('HighestFloor'),
                                                 True),
        'Lowest Neighbor Floor (lift)': (height('LowestFloor'),
                                         height('Floor'), False),
        'Raise 24 Units': (height('Floor', 24), None, False),
        'Raise 32 Units': (height('Floor', 32), None, False),
        'Raise Next Floor': (height('NextFloor'), None, False),
        'Stop': None,
    }

    print()
    print()
    print('### Platforms ###')
    print()
    for row in split_chunk(chunk, 9):
        special_type = to_special_type(row[0])
        extended = to_extended(row[1])
        trigger, only_once = to_trigger_and_only_once(row[2])
        wait = to_wait(row[3])
        speed = SPEEDS[row[4]]
        monsters = to_bool(row[7])

        triple = floors[row[8]]
        if triple is None:
            continue

        first, second, repeat = triple
        print('[[linedef]]')
        print('  special_type =', special_type)
        print('  trigger =', trigger)
        if extended:
            print('  extended = true')
        if only_once:
            print('  only_once = true')
        if monsters:
            print('  monsters = true')
        print('  [linedef.move]')
        if wait > 0.0:
            print('    wait =', wait)
        if speed > 0.0:
            print('    speed =', speed)
        if repeat:
            print('    repeat = true')
        if second is None:
            print('    floor = { first =', first, '}')
        else:
            print('    [linedef.move.floor]')
            print('      first =', first)
            print('      second =', second)
        print()


def exits(chunk):
    exits = {
        'Normal': '"Normal"',
        'Secret': '"Secret"',
    }

    print()
    print()
    print('### Exits ###')
    print()
    for row in split_chunk(chunk, 4):
        special_type = to_special_type(row[0])
        extended = to_extended(row[1])
        trigger, only_once = to_trigger_and_only_once(row[2])
        exit = exits[row[3]]

        print('[[linedef]]')
        print('  special_type =', special_type)
        print('  trigger =', trigger)
        if extended:
            print('  extended = true')
        if only_once:
            print('  only_once = true')
        print('  exit = ', exit)
        print()


def main():
    lines = [line.strip() for line in open('tables.txt', 'r')]

    def gen_chunks():
        chunk = []
        for line in lines:
            if line:
                chunk.append(line)
            else:
                yield chunk
                chunk = []

    chunks = gen_chunks()
    doors(next(chunks))
    floors(next(chunks))
    ceilings(next(chunks))
    platforms(next(chunks))
    next(chunks)  # crusher_ceilings(next(chunks))
    next(chunks)  # stair_builders(next(chunks))
    next(chunks)  # elevators(next(chunks))
    next(chunks)  # lighting(next(chunks))
    exits(next(chunks))
    next(chunks)  # teleporters(next(chunks))
    next(chunks)  # donuts(next(chunks))


if __name__ == '__main__':
    main()
