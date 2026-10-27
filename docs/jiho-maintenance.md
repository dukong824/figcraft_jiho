# figcraft_jiho 유지보수

이 저장소는 `storytold/figcraft`의 실제 Git 이력을 이어받은 공개 Fork입니다.
공식 기반 커밋은 `43af145a12eacd1e87f5f5e6984c49b7da040288`이며, 그 위에 Paper 이식과 개인 배포 변경을 별도 커밋으로 관리합니다.

## 공식 업데이트 받기

GitHub의 **Actions → Propose Upstream Update → Run workflow**를 실행합니다.
새 변경이 있으면 별도 브랜치에서 공식 `main`을 병합하고 검토용 PR을 만듭니다.
새 변경이 없으면 종료합니다. 충돌이 생기면 main을 바꾸지 않고 실패 원인을 기록합니다.
PR의 코드와 Jiho CI를 확인한 후 직접 병합합니다. 자동 승인·자동 병합·주기적 실행은 없습니다.

원본에서 새 workflow가 생겼는지도 검토하세요. 기존 공식 workflow는 `.github/upstream-workflows/`에 보관하고,
개인 배포에는 `.github/workflows/jiho-*.yml`만 사용합니다.

로컬에서 처리할 때:

```sh
git remote add upstream https://github.com/storytold/figcraft.git
git fetch upstream
git switch -c update-official-base
git merge upstream/main
cargo xtask ci
```

`upstream` remote가 이미 있으면 첫 줄은 생략합니다. 사용자 기능을 버리는 강제 동기화나 강제 푸시는 사용하지 않습니다.

## 새 Windows 빌드 배포

1. main의 CI와 기능을 확인합니다.
2. 아직 쓰지 않은 개인 태그(예: `jiho-v0.11.2.2`)를 main 커밋에 붙이고 push합니다.
3. **Jiho Windows Release**가 검사 후 Windows x64 portable ZIP과 SHA-256 파일을 prerelease로 게시합니다.

```sh
git tag jiho-v0.11.2.2
git push origin jiho-v0.11.2.2
```

태그 형식은 `jiho-v<공식 major>.<minor>.<patch>.<개인 빌드 번호>`입니다.
같은 태그를 다른 커밋으로 이동시키지 않습니다. 첫 공개 릴리스의 BUILD-INFO.txt에는 실제 빌드 프로필과 커밋을 기록합니다.

## 앱 하나로 사용하기

개인 빌드를 일정한 폴더에 두고 바로가기와 `.figcraft` 연결 프로그램을 `figcraft_jiho.exe`로 설정합니다.
업데이트 시 앱을 닫고 새 ZIP의 파일을 같은 위치에 교체하면 됩니다.
**도움말 → 업데이트 확인**은 이 Fork의 Releases 페이지를 안내합니다.
공식 설치 파일 자동 적용은 사용하지 않습니다. 개인 서명 체계를 갖춘 앱 내부 자동 교체는 아직 구현하지 않았습니다.

## 코드 관리 원칙

- Paper 계산식과 전처리는 별도 모듈을 유지하고, 원본 코드 접점은 작게 둡니다.
- 라이선스와 NOTICE, 출처를 유지합니다.
- 기능 수정과 공식 업데이트 병합, 개인 배포 변경을 서로 다른 커밋으로 남깁니다.
- 검사 실패를 숨기지 않습니다. 기존 보드 표 테스트의 환경 의존 실패는 VALIDATION.md에 기록되어 있습니다.
- 앱은 기존 FigCraft 설정·파일 형식을 기반으로 하며, 금속 효과 문서는 이 Fork로 열어야 합니다.
