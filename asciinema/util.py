def printf(s, *args):
    s = s.format(*args)
    print("\x1b[32m~ {}\x1b[0m".format(s))

def warningf(s, *args):
    s = s.format(*args)
    print("\x1b[33m~ {}\x1b[0m".format(s))
