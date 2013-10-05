import sys

from .config import Config
from .commands.builder import get_command

def main():
    get_command(sys.argv[1:], Config()).execute()

if __name__ == '__main__':
    main()
