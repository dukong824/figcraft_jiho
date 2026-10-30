# figcraft_jiho 유지보수

이 저장소는 `storytold/figcraft`의 실제 Git 이력을 이어받은 공개 Fork입니다.
공식 기반 커밋은 `43af145a12eacd1e87f5f5e6984c49b7da040288`이며, 그 위에 Paper 이식과 개인 배포 변경을 별도 커밋으로 관리합니다.

## 공식 업데이트 받기

GitHub의 **Actions → Propose Upstream Update → Run workflow**를 실행합니다.
새 변경이 있으면 별도 브랜치에서 공식 `main`을 병합하고 검토용 PR을 만듭니다.
자동 생성된 PR에도 검사가 실행되도록 해당 브랜치의 Jiho CI를 명시적으로 시작합니다.
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
- BiRefNet은 native 플랫폼 서비스로 분리합니다. 모델·런타임 버전과 SHA-256은 고정하고,
  교체 시 상업 이용 라이선스와 실제 추론을 다시 확인합니다. 다운로드 캐시는 앱 교체 후에도 유지됩니다.
- 라이선스와 NOTICE, 출처를 유지합니다.
- 기능 수정과 공식 업데이트 병합, 개인 배포 변경을 서로 다른 커밋으로 남깁니다.
- 검사 실패를 숨기지 않습니다. 기존 보드 표 테스트의 환경 의존 실패는 VALIDATION.md에 기록되어 있습니다.
- 앱은 기존 FigCraft 설정·파일 형식을 기반으로 하며, 금속 효과 문서는 이 Fork로 열어야 합니다.

## Purpose 아이콘

프로그램 소유자의 인터넷 아이콘 교체 요청에 따라 Purpose 전용 UI는 Lucide 1.64.0으로
통일합니다. 원본 SVG·전체 ISC/Feather MIT 고지·고정 커밋과 체크섬은 `assets/ui/lucide/`에
보관합니다. 원본 앱 로고는 소유자가 제공한 P 이미지를 유지합니다. FigCraft 단독 모드는
기존 자체 아이콘을 사용합니다. 같은 소스의 SVG를 다시 손으로 그려 추가하지 말고 공유
`figcraft-ui-icons` 렌더러와 원본·라이선스 기록을 함께 갱신하세요.


## Purpose Codex와 프로젝트 자료

Purpose의 Codex는 작업별 전용 MCP를 통해 실제 FigCraft/FilmCraft 명령과 라이브러리
서비스를 사용합니다. 원본 엔진의 고정 의존성과 FigCraft 단독 동작은 유지합니다.
대본과 Codex 가이드는 AI 도구에서 수정할 수 없고, 새 작업과 지침 변경 후에는 반드시
공통·채널 가이드를 읽어야 합니다. 도구는 요청한 영상에 고정하며 중지/완료 시 연결을
폐기합니다. CLI의 일반 파일 시스템을 쓰기 가능으로 바꾸거나 프로젝트 JSON을 직접
고치도록 유도해 이 경계를 우회하지 마세요.

라이브러리 제거는 자료의 논리적 휴지통 이동이며 기존 타임라인/대본 참조와 원본 파일을
보존합니다. 이름 변경은 표시 이름을 편집기 소스/기존 기본 클립 이름까지 전달합니다.
자료 가져오기에서 분석/대용량 복사는 백그라운드 서비스로 보내고 완료 시 원래 대상
영상에 등록합니다. 프로젝트 목록을 저장할 때 검증된 이전 JSON을 보관하며, 복구 중
손상된 원본은 별도 파일로 유지합니다.


## Purpose 0.1.10 UI와 기록 보관

문서/목록 페이지의 상단·제목·본문 폭과 메뉴/선택 메뉴는 `video_maker/style.rs`의 공통
기준을 사용합니다. 기본 글자 크기와 Ctrl+휠 배율을 따로 저장합니다. 폰트 변경 시 일반체를
굵은 폰트 패밀리 앞에 삽입하지 마세요. `Context` 스타일 변경 잠금 안에서 다시 Context를
읽으면 교착되므로 픽셀 배율/경계선 등은 잠금 밖에서 계산합니다.

기록은 `retention.rs`에서 영상별로 간격을 넓혀 30일까지 유지합니다. 물리적 정리는 현재·휴지통·
남은 기록·정상 복구 목록의 참조를 모두 확인하고, 1시간을 넘긴 미참조 `edit-<숫자>.fcproj`만
작업 스레드에서 삭제합니다. 심볼릭 링크나 미디어 파일로 정리를 확대하지 마세요.

편집기 소스 식별 시 `Session.media.follow`로 저장된 상대 경로를 먼저 해석합니다. 자동 bin
정리/가져오기/재연결은 실제 명령을 사용하되 기존 History를 보존합니다. 원본 제거·중복 정리는
보관 bin으로 이동해 클립 참조를 지키고, 사용자 Clear/Undo/Redo에 의한 자료 상태 변경만
라이브러리에 반영합니다. Codex 첨부는 영상/자료 ID로 저장하고, 현재 작업의 대상 영상은
변경하지 않습니다. 이미지 입력은 공식 app-server의 `localImage`를 사용합니다.


## Purpose 0.1.11 runtime and deletion rules

텍스트 크기는 `figcraft-ui-icons::with_font_scale`의 UI 프레임 범위에서만 적용합니다.
UI zoom에 글자 크기를 곱하지 마세요. 문서의 authored text 크기는 유지하며, 폰트 크기만
배율 함수를 거칩니다. 로고 행처럼 고정 너비를 계산할 때 마지막 버튼 앞의 item spacing도
예약하세요. 프레임 경계선은 수동으로 그리는 곳에서 Panel separator를 중복 표시하지 않습니다.

Codex CLI와 code-mode-host는 같은 버전/출처/체크섬으로 배포하고 함께 설치·실행 검사합니다.
다운로더 버전·고정 해시·전체 라이선스는 `downloader.ps1`과 `third-party/downloader/UPSTREAM.md`를
참고하세요. GPL optional dependencies가 포함된 upstream yt-dlp.exe로 바꾸지 마세요.

명시적인 휴지통 영구 삭제만 파일 정리를 확장합니다. 목록 변경과 pending_deletions를 먼저
저장한 뒤 소유 경로에서만 백그라운드 삭제합니다. 실패/종료 후 재시도하며 심볼릭 링크와
경로 이탈은 거부합니다. purged_assets 경로는 네이티브 소스 역가져오기 대상에서 제외해야 합니다.
일반 기록 retention은 여전히 미참조 .fcproj 스냅샷만 정리합니다.


## Purpose 0.1.12 approvals, paste and separators

채팅은 공식 Codex app-server의 stdio JSON-RPC를 사용합니다. 서버가 요청한 승인 ID와
원래 영상에 연결된 응답 채널을 유지하며, UI의 명시적 결정 전에는 응답하지 않습니다.
중지/완료 시 대기 요청을 폐기합니다. read-only sandbox와 guide-gated 앱 MCP 경계를
유지하고, 네트워크/읽기 권한은 요청한 turn 범위로만 승인합니다. 일반 파일 쓰기 권한을
허용하거나 대본/가이드를 도구로 수정하게 바꾸지 마세요. 설치 검사는 별도 CODEX_HOME의
initialize/thread-start까지만 실행하며 실제 유료 모델 turn을 시작하지 않습니다.

클립보드 이미지는 네이티브 PasteImage 이벤트로 받아 별도 작업 스레드에서 PNG로 저장합니다.
Color32의 premultiplied alpha는 저장 전에 unpremultiply하며, 자료 ID를 먼저 저장하고 최초
영상/대화에 등록합니다. 탭 전환·취소·종료·영구 삭제와 대기 작업의 소유권을 유지하세요.

사이드바/제목줄/오른쪽 패널 구분선은 마지막 shell mesh pass에서 물리 1픽셀 사각형으로
그립니다. 이 선을 antialias stroke로 바꾸거나 각 패널 배경 아래에서 그리지 마세요.
모달·팝업보다 낮은 Middle 레이어를 사용합니다. 문서 상단 여백은 DOCUMENT_TOP,
목록/다운로더 여백은 PAGE_TOP을 공통으로 사용합니다.
