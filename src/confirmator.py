import sys


class Confirmator(object):

    def confirm(self, text):
        sys.stdout.write(text)
        answer = sys.stdin.readline().strip()
        return answer == 'y' or answer == 'Y' or answer == ''
