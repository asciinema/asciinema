try:
    from setuptools import setup
except ImportError:
    from distutils.core import setup

config = {
    'name': 'asciinema',
    'version': '0.9.5',
    'packages': ['asciinema', 'asciinema.commands'],
    'license': 'MIT',
    'description': 'Command line recorder for asciinema.org service',
    'author': 'Marcin Kulik',
    'author_email': 'm@ku1ik.com',
    'url': 'http://asciinema.org',
    'download_url': 'https://github.com/sickill/asciinema/archive/v0.9.5.tar.gz',
    'install_requires': [],
    'entry_points': {
        'console_scripts': [
            'asciinema = asciinema.__main__:main',
        ],
    },
}

setup(**config)
