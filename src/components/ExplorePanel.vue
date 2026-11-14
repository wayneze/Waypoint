<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import type { ExploreRole, ExploreScope, ExploreSite } from '../services/exploreSites';
import {
  EXPLORE_ROLES,
  exploreQuery,
  exploreTerms,
  loadExploreSites,
  newExploreSite,
  saveExploreSites,
  siteUrl,
  siteSearchLabel,
  isHttp,
} from '../services/exploreSites';
import { openResourceWeb, searchExploreSite, type ExploreHit } from '../services/systemApi';
import { describeError } from '../services/errors';
import { showToast } from '../workspace/core';
import { singleAndGalgameGuides } from '../workspace/guides/library';
import type { ObsidianGameGuide } from '../types';
import { ArrowUpRight, BookOpen, ExternalLink, Plus, Search, Settings2, Trash2, X } from '@lucide/vue';
import ResourcePreview from './ResourcePreview.vue';
import { gameSourceIds } from '../services/gameLibrary';
import { linksForGames, removeGameLink, resourceDownloadError, type GameLink } from '../workspace/guides/extras';
import { handleOpenGuideReader, handleOpenResourceReader } from '../workspace/guides/reader';
const props = defineProps<{ game?: ObsidianGameGuide; initialRole?: ExploreRole; embedded?: boolean }>();
const selected = ref<GameLink>();
const preview = ref<HTMLElement>();
watch(selected, link => {
  if (link) preview.value?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
}, { flush: 'post' });
const collection = ref<'search' | 'saved'>('search');
const savedLinks = computed(() => picked.value ? linksForGames(gameSourceIds(picked.value)) : []);

const sites = ref<ExploreSite[]>([]);
const picked = ref<ObsidianGameGuide | null>(null);
onMounted(() => {
  sites.value = loadExploreSites();
});
const query = ref('');
const live = ref<Record<string, { state: 'loading' | 'ready' | 'empty' | 'failed'; hits: ExploreHit[]; error?: string; term?: string }>>({});
let searchToken = 0;
const scope = ref<ExploreScope>('galgame');
const editing = ref(false);
const managing = ref(false);
const draft = ref({ name: '', home: '', search: '', role: 'resource' as ExploreRole, scope: 'galgame' as ExploreScope });


const roleTab = ref<ExploreRole>(props.initialRole || 'guide');
const sourceFilter = ref('all');
watch(() => props.game, game => { if (game) { picked.value = game; collection.value = props.initialRole ? 'search' : 'saved'; query.value = exploreQuery(game); scope.value = game.category === 'galgame' ? 'galgame' : 'single'; selected.value = undefined; } }, { immediate: true });
async function removeSaved(link: GameLink) {
  if (!picked.value) return;
  try {
    await removeGameLink(gameSourceIds(picked.value), link.url);
    if (selected.value?.url === link.url) selected.value = undefined;
  } catch (e) { showToast('资料未移除', String(e)); }
}
function saved() { collection.value = 'saved'; }

const groups = computed(() => EXPLORE_ROLES.map(role => ({
  ...role,
  sites: sites.value
    .filter(site => site.role === role.id && (site.scope === scope.value || role.id === 'tool'))
    .sort((a, b) => a.order - b.order),
})));

const hits = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return [];
  return singleAndGalgameGuides.value
    .filter(guide => {
      const cat = guide.category === 'single' || guide.category === 'steam' ? 'single' : 'galgame';
      if (cat !== scope.value) return false;
      const names = [guide.title, guide.original_title || '', ...(guide.aliases || [])].join(' ').toLowerCase();
      return names.includes(q);
    })
    .slice(0, 6);
});

function persist() {
  saveExploreSites(sites.value);
}

function activeGuide(): ObsidianGameGuide | undefined {
  const guide = picked.value;
  if (!guide) return undefined;
  const q = query.value.trim();
  if ([guide.title, exploreQuery(guide), ...(guide.aliases || [])].some(name => name.trim() === q)) return guide;
  return undefined;
}

function openSite(site: ExploreSite, text = query.value) {
  const url = siteUrl(site, text ? exploreTerms(site, text, activeGuide())[0]! : '', activeGuide());
  openResourceWeb(url, picked.value?.title || site.name, picked.value?.record_id || picked.value?.id || '').catch(error => showToast('没打开', String(error)));
}

function sitesFor(role: ExploreRole) {
  return groups.value.find(group => group.id === role)?.sites.filter(site => !site.hidden) || [];
}

const resultSites = computed(() => sitesFor(roleTab.value).filter(site => site.result));
const visibleSites = computed(() => sitesFor(roleTab.value).filter(site=>sourceFilter.value==='all'||site.id===sourceFilter.value));
const articles = computed(() => visibleSites.value.flatMap(site=>(live.value[site.id]?.hits || []).map(hit=>({site,hit}))));
const pendingSites = computed(() => visibleSites.value.filter(site=>site.result && (!live.value[site.id] || live.value[site.id]?.state==='loading')));
const failedSites = computed(() => visibleSites.value.filter(site=>live.value[site.id]?.state==='failed'));
const readingGames = computed(()=>singleAndGalgameGuides.value.filter(g=>(g.category==='galgame'?'galgame':'single')===scope.value).slice(0,8));
watch([scope,roleTab],()=>{sourceFilter.value='all';selected.value=undefined;});

async function runLiveSearch(targets = resultSites.value) {
  window.clearTimeout(searchTimer);
  const token = targets === resultSites.value ? ++searchToken : searchToken;
  const q = query.value.trim();
  if (q.length < 2 || !targets.length) {
    live.value = {};
    return;
  }
  live.value = { ...live.value, ...Object.fromEntries(targets.map(site => [site.id, { state: 'loading' as const, hits: [] }])) };
  await Promise.all(targets.map(async site => {
    try {
      const guide = activeGuide();
      const terms = exploreTerms(site, q, guide);
      let hits: ExploreHit[] = [];
      let searched = q;
      for (const term of terms) {
        if (token !== searchToken) return;
        searched = term;
        live.value = { ...live.value, [site.id]: { state: 'loading', hits: [], term } };
        hits = await searchExploreSite(site.result!, term, site.home);
        if (hits.length) break;
      }
      if (token !== searchToken) return;
      live.value = { ...live.value, [site.id]: { state: hits.length ? 'ready' : 'empty', hits, term: searched } };
    } catch (error) {
      if (token !== searchToken) return;
      live.value = { ...live.value, [site.id]: { state: 'failed', hits: [], error: describeError(error).message } };
    }
  }));
}

let searchTimer = 0;
onBeforeUnmount(() => { window.clearTimeout(searchTimer); searchToken++; });
watch([query, scope, roleTab, collection, () => resultSites.value.map(site => `${site.id}:${site.home}:${site.result}`).join('|')], () => {
  window.clearTimeout(searchTimer);
  searchToken++;
  live.value = {};
  if (collection.value === 'search') searchTimer = window.setTimeout(() => void runLiveSearch(), 350);
}, { flush: 'sync' });

function openHit(hit: ExploreHit, site: ExploreSite) { selected.value = { title: hit.title, url: hit.url, site: site.name, role: site.role }; }

function useGuide(guide: ObsidianGameGuide) {
  picked.value = guide;
  query.value = exploreQuery(guide);
}

function toggleHidden(site: ExploreSite) {
  site.hidden = !site.hidden;
  persist();
}

function removeSite(site: ExploreSite) {
  sites.value = sites.value.filter(row => row.id !== site.id);
  persist();
}

function addSite() {
  const name = draft.value.name.trim();
  const home = draft.value.home.trim();
  if (!name || !isHttp(home)) {
    showToast('站点没写全', '要有名字，首页是 http 开头的地址');
    return;
  }
  const search = draft.value.search.trim();
  if (search && (!search.includes('{q}') || !isHttp(search.split('{q}').join('game')))) {
    showToast('搜索地址要留 {q}', '游戏名会填在 {q} 的位置');
    return;
  }
  sites.value.push(newExploreSite({
    name,
    home,
    search: search || undefined,
    role: draft.value.role,
    scope: draft.value.scope,
  }));
  persist();
  draft.value = { name: '', home: '', search: '', role: draft.value.role, scope: draft.value.scope };
  editing.value = false;
}

defineExpose({ query, scope });
</script>

<template>
  <section class="discover-workspace" :class="{'is-game':!!game,'has-preview':!!selected,'is-embedded':embedded}">
    <header class="discover-heading"><div><template v-if="!game"><p>中文攻略 · 日文路线 · 补丁与工具</p><h2 v-if="!embedded">攻略与资源<span>.</span></h2></template><nav v-else class="discover-collection" aria-label="作品资料收录"><button :aria-pressed="collection === 'saved'" @click="collection = 'saved'">已收录 {{ savedLinks.length + (game.file_path ? 1 : 0) }}</button><button :aria-pressed="collection === 'search'" @click="collection = 'search'">查找攻略与资源</button></nav></div><button type="button" class="discover-manage" :aria-pressed="managing" @click="managing = !managing; if (!managing) editing = false"><Settings2 />{{ managing ? '收起管理' : '管理来源' }}</button></header>
    <div v-if="picked && !game" class="discover-collection"><button :aria-pressed="collection === 'search'" @click="collection = 'search'">找资料</button><button :aria-pressed="collection === 'saved'" @click="collection = 'saved'">已收录 {{ savedLinks.length }}</button></div><p v-if="resourceDownloadError" class="discover-help" role="alert">{{ resourceDownloadError }}</p>
    <form v-if="collection === 'search'" class="discover-searchbar" @submit.prevent="runLiveSearch()"><div v-if="!game" class="discover-scope" aria-label="资料的游戏类别"><button v-for="item in [{id:'galgame' as const,label:'Galgame'},{id:'single' as const,label:'单机'}]" :key="item.id" type="button" :aria-pressed="scope === item.id" @click="scope = item.id">{{ item.label }}</button></div><label class="discover-search"><Search /><input v-model="query" type="search" aria-label="搜索攻略与资源" placeholder="搜索作品名、原名或关键词" /></label><button type="submit" class="discover-search-submit">搜索资料</button></form>
    <div v-if="!game && hits.length && collection === 'search'" class="discover-suggestions"><span>书库中</span><button v-for="guide in hits" :key="guide.id" type="button" @click="useGuide(guide)">{{ guide.title }}<ArrowUpRight /></button></div>
    <div v-if="game && collection === 'search'" class="discover-aliases"><span>搜索别名</span><button v-for="name in [...new Set([game.title,game.original_title,...(game.aliases || [])].filter(Boolean))].slice(0,6)" :key="name" @click="query = name!">{{ name }}</button></div>
    <nav v-if="collection==='search'" class="discover-roles" aria-label="搜索内容类型"><button v-for="role in EXPLORE_ROLES" :key="role.id" type="button" :aria-current="roleTab === role.id ? 'page' : undefined" @click="roleTab = role.id">{{ role.label }}</button></nav>
    <div v-if="collection==='search'" class="discover-sources" aria-label="筛选资料来源"><button :aria-pressed="sourceFilter==='all'" @click="sourceFilter='all'">全部来源</button><button v-for="site in sitesFor(roleTab)" :key="site.id" :aria-pressed="sourceFilter===site.id" @click="sourceFilter=site.id">{{ site.name }}<span v-if="live[site.id]?.hits.length">{{ live[site.id]?.hits.length }}</span></button></div>
    <div class="discover-split"><div class="discover-list-pane">
    <div v-if="collection === 'saved'" class="discover-saved">
      <article v-if="picked?.file_path"><button class="saved-title" @click="handleOpenGuideReader(picked)"><strong>本地攻略 · {{ picked.title }}</strong><small>Markdown · 阅读进度已保存</small></button><button @click="handleOpenGuideReader(picked)">继续阅读</button></article>
      <article v-for="link in savedLinks" :key="link.url"><button class="saved-title" @click="selected = link"><strong>{{ link.title }}</strong><small>{{ link.site }} · {{ link.article ? '正文已保存' : '在线来源' }}{{ link.local_files?.length ? ' · 已有本地文件' : '' }}</small></button><button v-if="link.article && picked" @click="handleOpenResourceReader(picked,link)">阅读</button><button :aria-label="'移除资料：' + link.title" @click="removeSaved(link)"><Trash2 /></button></article>
      <p v-if="!savedLinks.length && !picked?.file_path" class="discover-help">还没有收录攻略。<button class="discover-manage" @click="collection = 'search'">查找并收录到当前作品</button></p>
    </div>
    <template v-else-if="query.trim().length >= 2">
      <div class="resource-result-heading"><span>{{ articles.length }} 条资料</span><span v-if="pendingSites.length" role="status">正在搜索 {{ pendingSites.map(s=>s.name).join('、') }}…</span></div>
      <div class="resource-articles"><button v-for="{hit,site} in articles" :key="site.id+hit.url" class="resource-article" @click="openHit(hit,site)"><BookOpen /><span><strong>{{ hit.title }}</strong><small>{{ site.name }}<template v-if="hit.detail"> · {{ hit.detail }}</template></small></span><ArrowUpRight /></button></div>
      <p v-if="!articles.length && !pendingSites.length" class="resource-empty">{{ failedSites.length?'部分来源未能连接，可以重试或前往网站查看。':'没有找到匹配资料，试试作品原名或更短的关键词。' }}</p>
      <div v-if="failedSites.length" class="resource-failures"><div v-for="site in failedSites" :key="site.id"><span>{{ site.name }} · {{ live[site.id]?.error }}</span><button @click="runLiveSearch([site])">重试此来源</button></div></div>
      <div class="resource-web-links"><button v-for="site in visibleSites" :key="site.id" @click="openSite(site)">{{ site.result?'到':'搜索' }} {{ site.name }} {{ site.result?'查看':'' }}<ExternalLink /></button></div>
    </template>
    <div v-else class="resource-start"><BookOpen /><h3>为正在玩的作品找资料</h3><p>输入作品名，汇集各来源结果；点开预览后可收录到作品。</p><div v-if="readingGames.length" class="resource-library-picks"><span>从我的书库开始</span><button v-for="guide in readingGames" :key="guide.id" @click="useGuide(guide)">{{ guide.title }}<ArrowUpRight /></button></div><div class="resource-source-notes"><button v-for="site in visibleSites" :key="site.id" @click="openSite(site,'')"><strong>{{ site.name }}</strong><small>{{ siteSearchLabel(site) }}</small><ExternalLink /></button></div></div>
    </div><aside v-if="selected" ref="preview" class="discover-preview"><button type="button" class="discover-close-preview" aria-label="关闭资料预览" @click="selected = undefined"><X />{{ collection === 'saved' ? '返回已收录' : '返回结果' }}</button><ResourcePreview :link="selected" :game="picked" @navigate="selected = $event" @saved="saved" /></aside></div>
    <section v-if="managing" class="discover-management"><header><div><h3>我的站点</h3><p>调整{{ scope === 'galgame' ? ' Galgame ' : '单机' }}下的常用入口，设置保存在本机。</p></div><button type="button" class="discover-manage" :aria-expanded="editing" @click="editing = !editing"><Plus />{{ editing ? '收起添加' : '添加站点' }}</button></header><form v-if="editing" class="discover-form" @submit.prevent="addSite"><label>站点名称<input v-model="draft.name" required placeholder="名称" /></label><label>首页地址<input v-model="draft.home" required type="url" placeholder="https://" /></label><label class="discover-form-wide">搜索地址<input v-model="draft.search" placeholder="选填，用 {q} 代表游戏名" /></label><label>用途<select v-model="draft.role"><option v-for="role in EXPLORE_ROLES" :key="role.id" :value="role.id">{{ role.label }}</option></select></label><label>游戏类别<select v-model="draft.scope"><option value="galgame">Galgame</option><option value="single">单机</option></select></label><button type="submit">保存到本机</button></form><div class="discover-manage-groups"><section v-for="group in groups" :key="group.id"><h4>{{ group.label }}</h4><p v-if="!group.sites.length" class="discover-help">尚未添加站点</p><div v-for="site in group.sites" :key="site.id" class="discover-manage-row" :class="{ 'is-hidden':site.hidden }"><button type="button" @click="openSite(site,'')">{{ site.name }}<ExternalLink /></button><button type="button" :aria-label="(site.hidden ? '显示' : '隐藏') + site.name" @click="toggleHidden(site)">{{ site.hidden ? '显示' : '隐藏' }}</button><button type="button" :aria-label="'移除站点' + site.name" @click="removeSite(site)"><Trash2 /></button></div></section></div></section>
  </section>
</template>

<style scoped>
.discover-workspace.is-embedded{padding-top:6px}.is-embedded .discover-heading{margin-bottom:16px}.discover-search-submit{padding:10px 18px;border:1px solid var(--color-hairline);border-radius:4px;color:var(--color-beacon);font-size:12px;cursor:pointer}.discover-sources{display:flex;gap:8px;flex-wrap:wrap;margin:16px 0 24px}.discover-sources button{display:flex;align-items:center;gap:8px;padding:6px 10px;border:1px solid var(--color-hairline);border-radius:4px;color:var(--color-muted);font-size:11px;cursor:pointer}.discover-sources [aria-pressed=true]{background:var(--color-paper);border-color:var(--color-beacon);color:var(--color-beacon)}.discover-sources span{font:10px var(--font-mono)}.resource-result-heading{display:flex;justify-content:space-between;gap:20px;margin-bottom:8px;color:var(--color-faint);font-size:11px;line-height:1.8}.resource-article{display:flex;gap:14px;align-items:center;padding:18px 4px;border-bottom:1px solid var(--color-hairline);width:100%;text-align:left;cursor:pointer}.resource-article>span{flex:1;min-width:0}.resource-article svg{width:17px;height:17px;color:var(--color-beacon);flex:none}.resource-article strong{font-size:15px;font-weight:500;line-height:1.8;overflow-wrap:anywhere}.resource-article small{display:block;font-size:11px;color:var(--color-faint);margin-top:5px;line-height:1.7}.resource-article:hover{background:var(--color-paper)}.resource-failures{background:var(--color-paper);padding:12px 16px;margin-top:20px;border-radius:4px}.resource-failures>div{display:flex;align-items:baseline;gap:16px;justify-content:space-between;padding:5px 0;font-size:11px;color:var(--color-muted);line-height:1.8}.resource-failures button{flex:none;color:var(--color-beacon);cursor:pointer}.resource-web-links{display:flex;flex-wrap:wrap;gap:16px;margin:22px 0}.resource-web-links button{display:flex;gap:6px;align-items:center;font-size:11px;color:var(--color-muted);cursor:pointer}.resource-web-links svg{width:12px;height:12px}.resource-empty{font-size:13px;color:var(--color-muted);padding:30px 0}.resource-start{padding:20px 0}.resource-start>svg{width:26px;height:26px;color:var(--color-beacon);margin-bottom:16px}.resource-start h3{font:24px var(--font-display);margin-bottom:12px}.resource-start>p{font-size:12px;color:var(--color-muted);line-height:1.8}.resource-library-picks{display:flex;flex-wrap:wrap;gap:10px;margin:24px 0;max-width:1000px}.resource-library-picks>span{width:100%;font-size:11px;color:var(--color-faint);margin-bottom:3px}.resource-library-picks button{display:flex;align-items:center;gap:8px;max-width:100%;padding:8px 12px;border:1px solid var(--color-hairline);border-radius:4px;text-align:left;overflow-wrap:anywhere;font-size:12px;color:var(--color-muted);cursor:pointer}.resource-library-picks svg{width:13px;height:13px;flex:none}.resource-source-notes{display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:16px;margin-top:30px}.resource-source-notes button{position:relative;text-align:left;padding:16px 32px 16px 0;border-top:1px solid var(--color-hairline);cursor:pointer}.resource-source-notes strong{font-size:13px;font-weight:500;color:var(--color-muted)}.resource-source-notes small{display:block;font-size:11px;color:var(--color-faint);margin-top:7px}.resource-source-notes svg{position:absolute;right:5px;top:18px;width:13px;height:13px;color:var(--color-faint)}
.discover-close-preview{display:flex;align-items:center;gap:6px;margin-bottom:16px;font-size:11px;color:var(--color-muted);cursor:pointer}.discover-close-preview svg{width:14px;height:14px}.discover-preview{scroll-margin-top:70px}@media(max-width:1050px){.discover-preview{order:-1}}
.discover-workspace.is-game{padding-top:0}.discover-collection{display:flex;gap:8px;margin-bottom:18px}.discover-collection button{padding:7px 12px;font-size:12px;color:var(--color-muted);cursor:pointer;border-radius:4px}.discover-collection [aria-pressed=true]{color:var(--color-beacon);background:color-mix(in srgb,var(--color-beacon) 10%,transparent)}.discover-aliases{display:flex;flex-wrap:wrap;gap:6px 12px;font-size:11px;color:var(--color-faint);margin:12px 0}.discover-aliases button{color:var(--color-muted);cursor:pointer}.discover-split{display:grid;grid-template-columns:1fr;gap:28px}.has-preview .discover-split{grid-template-columns:minmax(240px,.8fr) minmax(0,1.2fr)}.is-game.has-preview .discover-split{grid-template-columns:1fr}.is-game .discover-preview{border-left:0;padding-left:0}.is-game .discover-heading{margin-bottom:16px;gap:10px}.is-game .discover-collection{margin-bottom:0;flex-wrap:wrap}.is-game .discover-collection button{padding:6px 8px}.is-game .discover-roles{flex-wrap:wrap;gap:12px;overflow:visible}.discover-list-pane{min-width:0}.discover-preview{border-left:1px solid var(--color-hairline);padding-left:26px;min-width:0}.discover-saved article{display:flex;align-items:center;gap:10px;padding:12px 0;border-bottom:1px solid var(--color-hairline)}.discover-saved button{font-size:12px;color:var(--color-beacon);cursor:pointer;padding:5px;flex-shrink:0}.discover-saved .saved-title{flex:1;min-width:0;text-align:left;color:var(--color-ink)}.discover-saved strong{font-size:13px;font-weight:500;overflow-wrap:anywhere}.discover-saved small{display:block;font-size:11px;color:var(--color-muted);margin-top:5px}.discover-saved svg{width:14px;height:14px}@media(max-width:1050px){.has-preview .discover-split{grid-template-columns:1fr}.discover-preview{border-left:0;border-top:1px solid var(--color-hairline);padding:24px 0 0}}

.discover-workspace { padding:26px 0 10px; }
.discover-heading { display:flex; justify-content:space-between; align-items:center; gap:20px; margin-bottom:25px; }
.discover-heading p { font-size:11px; color:var(--color-faint); }
.discover-heading h2 { font-family:var(--font-display); font-size:27px; font-weight:400; margin-top:6px; }
.discover-heading h2 span { color:var(--color-beacon); }
.discover-manage { display:inline-flex; align-items:center; justify-content:center; gap:6px; min-height:32px; font-size:11px; color:var(--color-muted); cursor:pointer; white-space:nowrap; }
.discover-manage svg { width:14px; height:14px; }
.discover-manage[aria-pressed='true'] { color:var(--color-beacon); }
.discover-searchbar { display:flex; align-items:center; gap:20px; margin-bottom:16px; }
.discover-scope { display:flex; align-items:center; gap:4px; background:var(--color-paper); padding:3px; border-radius:5px; flex-shrink:0; }
.discover-scope button { padding:7px 15px; border-radius:3px; font-size:12px; color:var(--color-faint); cursor:pointer; }
.discover-scope button[aria-pressed='true'] { background:var(--color-raised); color:var(--color-ink); }
.discover-search { display:flex; align-items:center; gap:10px; flex:1; min-width:0; padding:10px 12px; background:var(--color-chrome); border:1px solid var(--color-hairline); border-radius:5px; }
.discover-search svg { width:15px; height:15px; color:var(--color-faint); flex-shrink:0; }
.discover-search input { width:100%; min-width:0; background:transparent; outline:none; font-size:12px; }
.discover-search:focus-within { border-color:var(--color-beacon); }
.discover-suggestions { display:flex; align-items:center; gap:8px 16px; flex-wrap:wrap; margin:0 0 20px; }
.discover-suggestions > span { color:var(--color-faint); font-size:10px; }
.discover-suggestions button { display:flex; align-items:center; gap:5px; color:var(--color-muted); font-size:11px; cursor:pointer; }
.discover-suggestions svg { width:12px; height:12px; }
.discover-manage-groups { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:28px 38px; margin-top:28px; }
.discover-roles { display:flex; gap:20px; border-bottom:1px solid var(--color-hairline); margin-top:23px; overflow-x:auto; }
.discover-roles button { font-size:12px; color:var(--color-faint); padding:10px 0; border-bottom:2px solid transparent; cursor:pointer; white-space:nowrap; }
.discover-roles button[aria-current] { color:var(--color-beacon); border-bottom-color:currentColor; }
.discover-management { margin-top:35px; padding-top:24px; border-top:1px solid var(--color-hairline); }
.discover-management > header { display:flex; justify-content:space-between; align-items:center; gap:20px; }
.discover-management h3 { font-size:16px; }
.discover-management header p { color:var(--color-faint); font-size:11px; margin-top:5px; }
.discover-manage-groups h4 { color:var(--color-muted); font-size:12px; margin-bottom:10px; }
.discover-manage-row { display:flex; gap:12px; align-items:center; padding-block:9px; font-size:11px; }
.discover-manage-row.is-hidden { opacity:.55; }
.discover-manage-row button { display:inline-flex; align-items:center; gap:7px; cursor:pointer; color:var(--color-faint); }
.discover-manage-row button:first-child { margin-right:auto; color:var(--color-muted); text-align:left; }
.discover-manage-row svg { width:12px; height:12px; }
.discover-form { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:15px; margin-top:20px; padding:20px; background:var(--color-paper); border-radius:5px; }
.discover-form label { display:flex; flex-direction:column; gap:7px; font-size:11px; color:var(--color-muted); }
.discover-form-wide { grid-column:1/-1; }
.discover-form input, .discover-form select { width:100%; min-width:0; padding:8px 10px; border:1px solid var(--color-hairline); border-radius:4px; background:var(--color-chrome); color:var(--color-ink); font-size:12px; }
.discover-form > button { justify-self:start; padding:7px 12px; font-size:12px; color:var(--color-beacon); border:1px solid color-mix(in srgb,var(--color-beacon) 35%,transparent); border-radius:4px; cursor:pointer; }
button:hover { color:var(--color-beacon); }
:where(button,input,select):focus-visible { outline:2px solid var(--color-beacon); outline-offset:3px; }
.discover-search input:focus-visible { outline:none; }
@container (max-width:650px) { .discover-heading h2 { font-size:23px; } .discover-searchbar { flex-wrap:wrap; gap:12px; } .discover-search { flex-basis:100%; } .discover-manage-groups { grid-template-columns:1fr; gap:22px; } }
</style>
