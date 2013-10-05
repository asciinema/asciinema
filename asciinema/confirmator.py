from __future__ import print_function
import sys


class Confirmator(object):

    def confirm(self, text):
        print(text, end='')
        sys.stdout.flush()
        answer = sys.stdin.readline().strip()
        return answer == 'y' or answer == 'Y' or answer == ''
