try:
    from setuptools import setup
except ImportError:
    from distutils.core import setup

requirements = [
    'requests>=2.0.0'
]

setup(
    name='asciinema',
    version='0.9.5',
    packages=['asciinema', 'asciinema.commands'],
    license='MIT',
    description='Command line recorder for asciinema.org service',
    author='Marcin Kulik',
    author_email='m@ku1ik.com',
    url='http://asciinema.org',
    download_url='https://github.com/sickill/asciinema/archive/v0.9.5.tar.gz',
    entry_points={
        'console_scripts': [
            'asciinema = asciinema.__main__:main',
        ],
    },
    install_requires=requirements,
    classifiers=[
        'Development Status :: 4 - Beta',
        'Environment :: Console',
        'Intended Audience :: Developers',
        'Intended Audience :: System Administrators',
        'License :: OSI Approved :: MIT License',
        'Natural Language :: English',
        'Programming Language :: Python :: 2.6',
        'Programming Language :: Python :: 2.7',
        'Programming Language :: Python :: 3.2',
        'Programming Language :: Python :: 3.3',
        'Topic :: System :: Shells',
        'Topic :: Terminals',
        'Topic :: Utilities'
    ],
)
