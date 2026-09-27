# Ticker logo sources

XYZ market coverage checked on 2026-09-26 using Hyperliquid's public
[`info` endpoint](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint)
(`meta` with `dex: xyz` and `perpConciseAnnotations`). The existing and added
assets cover all 93 active stock/ETF markets and all 8 active commodity markets
in that snapshot. OURA's company logo is also included.

This addition contains 84 SVG files, including exchange/display ticker aliases.
Ticker filenames are resolved and embedded automatically by
`src/helpers/symbols.rs`. Existing logos continue to cover the other markets.

The files contain vector geometry, with transparent backgrounds and monochrome
paint for the app's theme tint. Export padding and editor metadata were removed.
Where no standalone fund logo was available, ETFs share their issuer's logo.
Smartbird (BIRD), Tradr (SNXX), and USA Rare Earth (USAR) were traced from their
published PNG artwork; these are local vector conversions, not issuer SVG files.

Oil markets use a barrel and natural gas uses a flame from
[Material Design Icons](https://github.com/Templarian/MaterialDesign-SVG).
Copper, silver, platinum and palladium use the existing simple bullion-bars
icon also used for gold.

Source catalog license notices are bundled in `assets/licenses/`:
[Simple Icons](../../assets/licenses/simple-icons.txt),
[SVGL](../../assets/licenses/svgl.txt), and
[Material Design Icons](../../assets/licenses/material-design-icons.txt).
Company marks belong to their respective owners; the catalog license does not
imply ownership of the brands.

## Added files

All ticker names below correspond to `assets/<TICKER>.svg`.

| Tickers | Company / icon | Source | Notes |
| --- | --- | --- | --- |
| `AAOI` | Applied Optoelectronics | [Issuer website](https://ao-inc.com/) · [artwork](https://ao-inc.com/_resources/themes/appliedopto/images/logo-white.svg?m=1778852878) | — |
| `AMAT` | Applied Materials | [Vector logo catalog](https://www.logo.wine/logo/Applied_Materials) · [artwork](https://www.logo.wine/a/logo/Applied_Materials/Applied_Materials-Logo.wine.svg) | — |
| `ARM` | Arm | [Simple Icons](https://www.arm.com) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/arm.svg) | — |
| `ASML` | ASML | [Issuer website](https://www.asml.com/en) · [artwork](https://www.asml.com/images/icons/asml-logo.svg) | Standalone SVG extracted from the issuer SVG symbol. |
| `AVGO` | Broadcom | [Simple Icons](https://www.broadcom.com/support) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/broadcom.svg) | — |
| `BB` | Blackberry | [Simple Icons](https://www.blackberry.com) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/blackberry.svg) | — |
| `BE` | Bloom Energy | [CompaniesLogo](https://companieslogo.com/bloom-energy/logo/) · [artwork](https://companieslogo.com/img/orig/BE-7207020b.svg?t=1720244491&download=true) | — |
| `BIRD` | Smartbird | [Issuer website](https://www.smartbird.ai/) · [artwork](https://images.squarespace-cdn.com/content/v1/6a0cc263a1dff83a2a199556/f78dfd0b-4647-41e8-976f-0a7fc07fb443/Smartbird_BK_Icon.png) | Traced from issuer PNG; monochrome paths |
| `BMNR` | BitMine | [CompaniesLogo](https://companieslogo.com/bitmine-immersion-technologies/logo/) · [artwork](https://companieslogo.com/img/orig/BMNR-bbabe2e2.svg?t=1767789704&download=true) | B glyph; gradient background removed. |
| `BOT` | RoboStrategy | [Issuer website](https://robostrategy.co/) | — |
| `BX` | Blackstone | [CompaniesLogo](https://companieslogo.com/blackstone-group/logo/) · [artwork](https://companieslogo.com/img/orig/BX-61e2cf13.svg?t=1720244491&download=true) | Wordmark; background removed. |
| `CL`, `WTIOIL`, `BRENTOIL` | Oil barrel | [Material Design Icons](https://github.com/Templarian/MaterialDesign-SVG/blob/master/svg/barrel.svg) | Shared oil-barrel icon for raw CL, displayed WTIOIL and BRENTOIL. |
| `COIN` | Coinbase | [Simple Icons](https://www.coinbase.com/press) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/coinbase.svg) | — |
| `COST` | Costco | [Issuer website](https://www.costco.com/) · [artwork](https://azure-na-images.contentstack.com/v3/assets/blt79a1ca72a67c8924/blt8c4d2e413e2cb617/64e3b60e88e161d1c0fce933/costco-app-icon-32x32.svg) | C glyph; app tile and shadow removed. |
| `CRWD` | CrowdStrike | [Issuer website](https://www.crowdstrike.com/) · [artwork](https://assets.crowdstrike.com/is/content/crowdstrikeinc/CS_Logo_Falcon) | — |
| `CRWV` | CoreWeave | [Issuer website](https://www.coreweave.com/) · [artwork](https://cdn.prod.website-files.com/62ba1fb86485b6d5029975c4/698cca4beeec0fe985632156_logo_coreweave_solo_white.svg) | — |
| `CVX` | Chevron | [Vector logo catalog](https://www.logo.wine/logo/Chevron_Corporation) · [artwork](https://www.logo.wine/a/logo/Chevron_Corporation/Chevron_Corporation-Logo.wine.svg) | — |
| `CXMT` | ChangXin Memory Technologies | [Issuer website](https://www.cxmt.com/) · [artwork](https://www.cxmt.com/statics/shuwon/assets/img/svg/logo.svg) | — |
| `DELL` | Dell | [Simple Icons](https://www.dell.com) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/dell.svg) | — |
| `DKNG` | DraftKings | [Issuer website](https://www.draftkings.com/) | Crown glyph; background removed. |
| `DRAM` | Roundhill DRAM | [Issuer website](https://www.roundhillinvestments.com/etf/dram/) · [artwork](https://www.roundhillinvestments.com/assets/img/dram_fund_logo.svg) | ETF fund. |
| `EBAY` | eBay | [Simple Icons](https://go.developer.ebay.com/logos) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/ebay.svg) | — |
| `EWY`, `EWJ`, `EWT`, `EWZ`, `TLT` | iShares | [Issuer website](https://www.ishares.com/us) · [artwork](https://www.ishares.com/blk-one-c-assets/images/media-bin/web/global/logos/logo-ishares.svg) | ETF issuer. |
| `GEV` | GE Vernova | [Issuer website](https://www.gevernova.com/) · [artwork](https://www.gevernova.com/themes/custom/ge_vernova_unified/logo.svg) | — |
| `GIGADEV` | GigaDevice | [Company website](https://www.gigadevice.com/) · [artwork](https://www.gigadevice.com/Public/Uploads/uploadfile/images/20220517/logo1-330.svg) | — |
| `GME` | GameStop | [Vector logo catalog](https://companieslogo.com/gamestop/logo/) · [artwork](https://companieslogo.com/img/orig/GME-7953e22b.svg?t=1720244492) | — |
| `HYUNDAI` | Hyundai | [Simple Icons](https://www.hyundai.com) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/hyundai.svg) | — |
| `IBM` | IBM | [SVGL](https://www.ibm.com) · [artwork](https://raw.githubusercontent.com/pheralb/svgl/main/static/library/ibm.svg) | — |
| `IREN` | IREN | [Issuer website](https://iren.com/) · [artwork](https://iren.com/icons/logo.svg) | Original block outline with transparent letter cutouts. |
| `KIOXIA` | Kioxia | [Issuer website](https://www.kioxia-holdings.com/en-jp/) · [artwork](https://www.kioxia-holdings.com/etc.clientlibs/kioxia-libs/clientlibs/kioxia/resources/component/kioxia_logo.svg) | — |
| `KORU`, `SOXL` | Direxion | [CompaniesLogo](https://companieslogo.com/direxion-etf/logo/) · [artwork](https://companieslogo.com/img/orig/direxion-77e1f031.svg?t=1720244494&download=true) | ETF issuer. |
| `KSTR` | KraneShares | [CompaniesLogo](https://companieslogo.com/kraneshares/logo/) · [artwork](https://companieslogo.com/img/orig/KraneShares-81b59177.svg?t=1720244492&download=true) | ETF issuer. |
| `LITE` | Lumentum | [Issuer website](https://www.lumentum.com/en) · [artwork](https://www.lumentum.com/images/logos/Logo.svg) | — |
| `LYTE` | Roundhill LYTE | [Issuer website](https://www.roundhillinvestments.com/etf/lyte/) · [artwork](https://www.roundhillinvestments.com/assets/img/lyte_fund_logo.svg) | ETF fund. |
| `MAGS` | Roundhill MAGS | [Issuer website](https://www.roundhillinvestments.com/etf/mags/) · [artwork](https://www.roundhillinvestments.com/assets/img/mags_fund_logo.svg) | ETF fund. |
| `MINIMAX` | MiniMax | [Simple Icons](https://github.com/MiniMax-AI/MiniMax-01/blob/57cf223b177e99636c7711a0f179e9fdc9c38e8a/figures/minimax.svg) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/minimax.svg) | — |
| `MRNA` | Moderna | [Issuer website](https://www.modernatx.com/) · [artwork](https://www.modernatx.com/assets/favicons/safari-pinned-tab.svg) | — |
| `MRVL` | Marvell | [CompaniesLogo](https://companieslogo.com/marvell/logo/) · [artwork](https://companieslogo.com/img/orig/MRVL-cd40f6c4.svg?t=1720244492&download=true) | — |
| `MSTR`, `STRC` | Strategy | [CompaniesLogo](https://companieslogo.com/microstrategy/logo/) · [artwork](https://companieslogo.com/img/orig/MSTR-2221948f.svg?t=1741969284&download=true) | Strategy issuer mark shared by MSTR and STRC. |
| `NATGAS` | Natural gas flame | [Material Design Icons](https://github.com/Templarian/MaterialDesign-SVG/blob/master/svg/fire.svg) | — |
| `NBIS` | Nebius | [Issuer website](https://nebius.com/) · [artwork](https://nebius.com/favicon/favicon.svg) | N glyph; background removed. |
| `NCLD` | Roundhill NCLD | [Issuer website](https://www.roundhillinvestments.com/etf/ncld/) · [artwork](https://www.roundhillinvestments.com/assets/img/ncld_fund_logo.svg) | ETF fund. |
| `NET` | Cloudflare | [Simple Icons](https://www.cloudflare.com/logo/) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/cloudflare.svg) | — |
| `NFLX` | Netflix | [Simple Icons](https://brand.netflix.com/en/assets/logos) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/netflix.svg) | — |
| `NOK` | Nokia | [Simple Icons](https://www.nokia.com) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/nokia.svg) | — |
| `NOW` | ServiceNow | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:ServiceNow_logo.svg) · [artwork](https://upload.wikimedia.org/wikipedia/commons/5/57/ServiceNow_logo.svg) | — |
| `OURA` | Oura | [Issuer website](https://ouraring.com/) | — |
| `PURRDAT` | Hyperliquid Strategies | [Company website](https://hypestrat.xyz/) · [artwork](https://hypestrat.xyz/favicon.svg) | — |
| `QCOM` | Qualcomm | [Simple Icons](https://www.qualcomm.com) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/qualcomm.svg) | — |
| `QNT` | Quantinuum | [Issuer website](https://www.quantinuum.com/) | — |
| `RDDT` | Reddit | [Simple Icons](https://www.redditinc.com/brand) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/reddit.svg) | — |
| `RIVN` | Rivian | [Issuer website](https://rivian.com/) | — |
| `RKLB` | Rocket Lab | [CompaniesLogo](https://companieslogo.com/rocket-lab-usa/logo/) · [artwork](https://companieslogo.com/img/orig/RKLB-7df1a455.svg?t=1720244493&download=true) | — |
| `SHAZ` | Sharon AI | [Issuer website](https://sharonai.com/) · [artwork](https://sharonai.com/wp-content/uploads/2026/07/Sharon-AI-preffered-logo.svg) | — |
| `SHEIN` | SHEIN | [Issuer website](https://www.sheingroup.com/) | — |
| `SILVER`, `COPPER`, `PLATINUM`, `PALLADIUM` | Metal bullion bars | [Existing Kerosene icon](../../assets/gold.svg) | Reuse the existing gold.svg bullion glyph for metals. |
| `SKHX`, `SKHY`, `SKHYNIX` | SK hynix | [Vector logo catalog](https://www.logo.wine/logo/SK_Hynix) · [artwork](https://www.logo.wine/a/logo/SK_Hynix/SK_Hynix-Logo.wine.svg) | Shared company logo for SKHX, SKHY and displayed SKHYNIX. |
| `SMH` | VanEck | [CompaniesLogo](https://companieslogo.com/vaneck-etf/logo/) · [artwork](https://companieslogo.com/img/orig/vaneck_BIG-fc1e6fe4.svg?t=1720244494&download=true) | ETF issuer. |
| `SMSN` | Samsung | [Kerosene](../../assets/samsung.svg) | Filename alias for the existing samsung.svg artwork. |
| `SNDK` | Sandisk | [Kerosene](../../assets/snkd.svg) | Filename alias for the existing snkd.svg artwork. |
| `SNXX` | Tradr | [Issuer website](https://www.tradretfs.com/snxx) · [artwork](https://www.tradretfs.com/hubfs/Tradr%20ETFs%20SHIELD%20Logo%20color%201.png) | ETF issuer. Traced from issuer PNG; monochrome paths |
| `SOFTBANK` | SoftBank Group | [Issuer website](https://group.softbank/en) · [artwork](https://group.softbank/media/Project/sbg/sbg/themes/custom/sbg/sbg-logo.svg?iar=0) | — |
| `TSM` | TSMC | [CompaniesLogo](https://companieslogo.com/tsmc/logo/) · [artwork](https://companieslogo.com/img/orig/TSM-f424c30e.svg?t=1722952500&download=true) | TSMC wordmark paths; wafer and separate baseline omitted for monochrome display. |
| `UNITREE` | Unitree | [Issuer website](https://www.unitree.com/) · [artwork](https://www.unitree.com/unitree-favicon.svg) | — |
| `URNM` | Sprott | [Issuer website](https://sprottetfs.com/urnm-sprott-uranium-miners-etf/) · [artwork](https://sprottetfs.com/Themes/Sprott_ETFs_2024/Assets/images/logo.svg) | ETF issuer. |
| `USAR` | USA Rare Earth | [Issuer website](https://www.usare.com/) · [artwork](https://www.usare.com/wp-content/uploads/2026/03/usare-main-logo.png) | Traced from issuer PNG; monochrome paths; mountain mark only |
| `WDC` | Western Digital | [CompaniesLogo](https://companieslogo.com/western-digital/logo/) · [artwork](https://companieslogo.com/img/orig/WDC-d9008470.svg?t=1774060104&download=true) | — |
| `XLE`, `XBI` | State Street Investment Management | [Issuer website](https://www.ssga.com/us/en/individual/etfs) · [artwork](https://www.ssga.com/library-content/images/site/state-street-investment-management-logo.svg) | ETF issuer. |
| `ZHIPU` | Zhipu | [Issuer website](https://www.zhipuai.cn/) · [artwork](https://www.zhipuai.cn/logo.svg) | — |
| `ZM` | Zoom | [Simple Icons](https://brand.zoom.us/media-library/) · [artwork](https://raw.githubusercontent.com/simple-icons/simple-icons/develop/icons/zoom.svg) | — |
