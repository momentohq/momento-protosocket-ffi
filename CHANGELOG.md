# Changelog

## [0.2.1](https://github.com/momentohq/momento-protosocket-ffi/compare/v0.2.0...v0.2.1) (2025-10-20)


### Bug Fixes

* some cleanup and upgrade rust sdk for bug fix ([#34](https://github.com/momentohq/momento-protosocket-ffi/issues/34)) ([3932bbb](https://github.com/momentohq/momento-protosocket-ffi/commit/3932bbb4dc3f6daaed7505db17ad5203e976b680))


### Miscellaneous

* Switch the FFI to a string-based API key instead of an env var ([#31](https://github.com/momentohq/momento-protosocket-ffi/issues/31)) ([b79f556](https://github.com/momentohq/momento-protosocket-ffi/commit/b79f556de8c5d22a3ac5970948c08c5a0fae76ae))

## [0.2.0](https://github.com/momentohq/momento-protosocket-ffi/compare/v0.1.1...v0.2.0) (2025-10-15)


### Features

* Initial callback implementation ([#26](https://github.com/momentohq/momento-protosocket-ffi/issues/26)) ([fc02da0](https://github.com/momentohq/momento-protosocket-ffi/commit/fc02da087d7e2d0a257747600ac246e740e963b3))

## [0.1.1](https://github.com/momentohq/momento-protosocket-ffi/compare/v0.1.0...v0.1.1) (2025-10-15)


### Miscellaneous

* remove directory structure from released tar files, add pkg-config template file ([#25](https://github.com/momentohq/momento-protosocket-ffi/issues/25)) ([868adc1](https://github.com/momentohq/momento-protosocket-ffi/commit/868adc1f93a9602ac6a887ad310917d4a6d58dff))

## 0.1.0 (2025-10-14)


### Features

* add examples directory with golang example ([9fcbea7](https://github.com/momentohq/momento-protosocket-ffi/commit/9fcbea72dad89eca1597d1d3385d432ff5121f4c))
* add FFI files ([fa0573c](https://github.com/momentohq/momento-protosocket-ffi/commit/fa0573c8da5e155cb8bc47a8eaba994d8ad553ea))
* add pkg-config template and standardize underscore usage ([#21](https://github.com/momentohq/momento-protosocket-ffi/issues/21)) ([d1f9ecb](https://github.com/momentohq/momento-protosocket-ffi/commit/d1f9ecb74fa8c992a5c7e378f7d6e8ae3ce9d146))


### Bug Fixes

* make sure to clean up all allocated resources ([#12](https://github.com/momentohq/momento-protosocket-ffi/issues/12)) ([dc30eb8](https://github.com/momentohq/momento-protosocket-ffi/commit/dc30eb8879dccd02a3188c1192b8178da6cc100b))
* set macosx deployment target to avoid tons of go build warnings ([#11](https://github.com/momentohq/momento-protosocket-ffi/issues/11)) ([f20bfb1](https://github.com/momentohq/momento-protosocket-ffi/commit/f20bfb1e3992013975968b1479c4a09b97371963))
* update sdk version to improve connection logic ([51421b1](https://github.com/momentohq/momento-protosocket-ffi/commit/51421b1bcdfd1a0c5e90e39a39cda955d3ebc763))
* update sdk version to improve connection logic ([ce74343](https://github.com/momentohq/momento-protosocket-ffi/commit/ce74343e4d195767d55d2a8c85d3d4620a1a5ba6))


### Miscellaneous

* add readme templates and generate in ci/cd ([#23](https://github.com/momentohq/momento-protosocket-ffi/issues/23)) ([85a615d](https://github.com/momentohq/momento-protosocket-ffi/commit/85a615ddfa2d4fc45a4a4ed4670935554b073afa))
* extra file with version number missing release-please comment ([#24](https://github.com/momentohq/momento-protosocket-ffi/issues/24)) ([8726b00](https://github.com/momentohq/momento-protosocket-ffi/commit/8726b000f80e80e2ebfe2e36ac796b97187ff40a))
* github actions setup ([#14](https://github.com/momentohq/momento-protosocket-ffi/issues/14)) ([1c07c38](https://github.com/momentohq/momento-protosocket-ffi/commit/1c07c38e944bcffe02c25cf516b304a2aeb4ae22))
* update basic repo files ([49e7f0b](https://github.com/momentohq/momento-protosocket-ffi/commit/49e7f0ba0fb6721cce70a5b6be23027b6281bcbd))
* update build matrix and set up release process ([#16](https://github.com/momentohq/momento-protosocket-ffi/issues/16)) ([cd04be9](https://github.com/momentohq/momento-protosocket-ffi/commit/cd04be90946b86123b38aab2eabdff32102ac754))
