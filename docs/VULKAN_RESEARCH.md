# Rust + Vulkan / GPU — дослідження для rebook (2026-09)

Питання: чи потрібен **Vulkan** для віртуального стенда книги
(ebook + paperback + hardcover)?

Коротко: **ні, не в цьому продукті.** Стенд уже є CSS 3D на `/view3d` з
точною KDP-геометрією. Сирий Vulkan зламав би ratio (unsafe + шар DLL) і
не дав би переваги читачу EPUB у браузері.

## Що є в екосистемі (стан 2026)

| Крейт | Рівень | Коли брати | Для rebook |
|-------|--------|------------|------------|
| **ash** 0.38 | raw Vulkan 1.4, `unsafe`, generated from `vk.xml` | власний рушій, нульова валідація | ні — немає валідації, Windows-gnu + Vulkan SDK окремо |
| **vulkano** 0.35 | safe wrapper над ash; taskgraph (2025) ще experimental | нативний Vulkan-додаток, не веб | ні — інший процес, інший UX |
| **wgpu** 24/25 | WebGPU; бекенди Vulkan / DX12 / Metal / GLES / WebGPU | портативний GPU, Bevy, Firefox | **єдиний кандидат**, якщо колись знадобиться native preview |
| **erupt** | raw Vulkan | maintainers радять **ash** | не брати |
| CSS 3D + SVG | браузер | книга як об'єкт з 3–5 гранями | **поточний канон** |

GSV/Telenetis уже мають WebGPU-карту в Mini App — не копіюємо цей стек у
rebook (інший продукт, інший ratio-бюджет: wasm 0–5 %).

## Рішення

1. **Залишити CSS 3D** для `/view3d` і cover mini-3D. Геометрія важливіша
   за шейдери: bleed, spine, hinge, flaps, barcode zone.
2. **Не додавати ash/vulkano** у `Cargo.toml`. Це порушить Rust-ratio
   (багато `unsafe`) і не рендериться в `include_str!` UI.
3. Якщо з'явиться native «вітрина» (окреме вікно, не :8090): **wgpu**, не
   ash. Один бекенд на Windows (DX12/Vulkan) + той самий код.
4. Raster-прев'ю сторінки друку вже рахує Studio page-view (колонки CSS =
   interior metrics). GPU тут нічого не прискорює.

Джерела: ash-rs/ash (Vulkan 1.4), vulkano.rs, Sparkles Vulkan survey 2025
(ash zero-cost / wgpu 5–10 % CPU vs raw HAL / vulkano taskgraph experimental).
