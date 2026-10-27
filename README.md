# figcraft_jiho

나만의 FigCraft Windows 빌드. 공식 FigCraft의 Git 이력을 유지하면서 개인 기능을 추가하는 공개 Fork입니다.

**[Windows 빌드 다운로드](https://github.com/dukong824/figcraft_jiho/releases)** · [공식 FigCraft](https://github.com/storytold/figcraft) · [유지보수 안내](docs/jiho-maintenance.md)

## 사용하기

Releases의 Windows x64 portable ZIP을 풀고 **`figcraft_jiho.exe`**를 실행합니다.
`Open Demo.cmd`로 금속 효과 데모를 열 수 있습니다.
`Open Image Filters.cmd`로 이미지 필터 7종과 원본을 비교할 수 있습니다.
기존 `.figcraft` 작업 파일을 이 앱으로 열어 계속 작업하면 됩니다.

앱을 일정한 폴더에 두고 Windows의 `.figcraft` 연결 프로그램과 바로가기를 이 실행 파일로 설정하세요.
새 버전이 나오면 앱을 닫고 같은 폴더의 파일을 교체합니다. 앱 내 업데이트 확인은 이 저장소의 Releases를 안내합니다.
개인 빌드에 공식 설치 파일을 자동 적용하지 않습니다. 앱 자체의 자동 교체는 아직 구현하지 않았습니다.

## 개인 기능

- **Paper Liquid Metal:** Paper가 공개한 GLSL 원본과 알파/Poisson 전처리를 이식한 네이티브 애니메이션 효과.
- 이미지·벡터·프레임을 선택하고 **디자인 → 커스텀 → + → 리퀴드 메탈**에서 색상, 속도, 반사 무늬를 조절합니다.
- **이미지 필터 7종:** 종이 질감, 플루티드 글라스, 물결, 이미지 디더링, 하프톤 도트, 하프톤 CMYK, 렌즈 왜곡. **커스텀 → + → 이미지 필터**에서 선택합니다. [기능 안내](docs/paper-image-filters.md)
- 커스텀은 채우기·선·효과와 같은 계층의 섹션입니다. 기존 파일의 금속 효과도 이곳에 표시됩니다.
- `Image to GIF.cmd`에 이미지를 끌어 놓으면 3초, 24fps GIF를 만듭니다.
- 기존 FigCraft 설정과 작업 파일을 기반으로 사용합니다. 새 효과가 포함된 문서는 이 Fork로 여세요.

Paper 원본은 **투명도에 따른 윤곽**을 사용합니다. 투명 PNG와 벡터에 잘 맞으며, 배경이 있는 사진은 사각형 금속 표면이 됩니다.
실시간 성능은 기기에 따라 달라집니다. 캔버스 효과 입력은 최대 512px, 저장용은 최대 1024px입니다.
웹 버전은 효과 설정을 보존하지만 이 네이티브 GPU 효과를 렌더링하지 않습니다.

## 공식 업데이트와 새 빌드

- **Actions → Propose Upstream Update:** 공식 소스를 별도 브랜치에 병합하고 검토용 PR을 만듭니다. 충돌 시 main은 바꾸지 않습니다.
- **Jiho CI:** 코드 검사, 테스트, 아키텍처·출처 검사, WASM 호환성을 확인합니다.
- **`jiho-v0.11.2.2` 같은 태그를 push:** 검사를 통과한 뒤 **Jiho Windows Release**가 새 Windows 빌드를 prerelease로 배포합니다.

공식 업데이트의 자동 승인·자동 병합·주기적 실행은 없습니다. 자세한 절차는 [유지보수 안내](docs/jiho-maintenance.md)를 참고하세요.

## 소스와 검증

공식 기반: `43af145a12eacd1e87f5f5e6984c49b7da040288`.
추가 기능을 별도 커밋으로 유지하므로 이후 공식 변경을 Git merge로 반영할 수 있습니다.

[Paper 구현 범위](docs/paper-liquid-metal.md) · [검증 기록과 알려진 제한](VALIDATION.md) · [원본 README](README.upstream.md)

첫 `preview-v…` 공개 패키지는 로컬에서 검사한 실험용 빌드이며 실제 컴파일 프로필은 ZIP의 `BUILD-INFO.txt`에 기록합니다.
이후 태그 기반 배포는 GitHub Actions의 release 빌드를 사용합니다.

## 라이선스

FigCraft: MIT OR Apache-2.0. 원본 저작권·NOTICE·출처를 유지합니다.
Paper Shaders: Apache-2.0, Copyright Lost Coast Labs, Inc.
**Powered by [Paper Shaders](https://shaders.paper.design).**

공식 FigCraft/ArtCraft와 별도로 유지하는 개인 Fork입니다. 포함된 원본 상표·로고의 권리는 원저작자에게 있습니다.
