import sys

if sys.version_info[0] < 3:
    sys.exit('Python < 3 is unsupported.')

try:
    from setuptools import setup
except ImportError:
    from distutils.core import setup

import asciinema


url_template = 'https://github.com/asciinema/asciinema/archive/v%s.tar.gz'
requirements = []

setup(
    name='asciinema',
    version=asciinema.__version__,
    packages=['asciinema', 'asciinema.commands'],
    license='GNU GPLv3',
    description='Terminal session recorder',
    author=asciinema.__author__,
    author_email='m@ku1ik.com',
    url='https://asciinema.org',
    download_url=(url_template % asciinema.__version__),
    entry_points={
        'console_scripts': [
            'asciinema = asciinema.__main__:main',
        ],
    },
    install_requires=requirements,
    classifiers=[
        'Development Status :: 5 - Production/Stable',
        'Environment :: Console',
        'Intended Audience :: Developers',
        'Intended Audience :: System Administrators',
        'License :: OSI Approved :: GNU General Public License v3 or later (GPLv3+)',
        'Natural Language :: English',
        'Programming Language :: Python',
        'Programming Language :: Python :: 3',
        'Programming Language :: Python :: 3.2',
        'Programming Language :: Python :: 3.3',
        'Programming Language :: Python :: 3.4',
        'Programming Language :: Python :: 3.5',
        'Topic :: System :: Shells',
        'Topic :: Terminals',
        'Topic :: Utilities'
    ],
)
