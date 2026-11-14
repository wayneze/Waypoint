import { computed, ref, watch } from 'vue';
import { kvGet, kvSet } from '../../services/kv';
import { state, showToast } from '../core';
import { activeSkyCharacter } from '../vault';
import { skyCollectionEntries, setSkyCollectionOwned, setSkyCollectionWanted, type SkyCollectionEntry } from '../../services/skyCollection';
import { collectionWear } from '../../services/skyCollectionMedia';

export const spiritCollectionSource = ref<'permanent' | 'season'>('permanent');
export const collectionImageMode = ref<'icon' | 'wear'>('wear');
export function restoreCollectionImageMode() { collectionImageMode.value = kvGet('sky.collection.imageMode') === 'icon' ? 'icon' : 'wear'; }
watch(collectionImageMode, mode => kvSet('sky.collection.imageMode', mode));
export const collectionStatus = ref<'owned' | 'all' | 'wanted' | 'missing'>('owned');
export const collectionSource = ref('all');
export const collectionSearch = ref('');
export const collectionKind = ref('');
export const savingCollection = ref(false);
export const collectionError = ref('');
export const collectionCanEdit = computed(() => !!activeSkyCharacter.value && !savingCollection.value);
export const collectionEntries = computed(() => skyCollectionEntries(activeSkyCharacter.value?.id || '', state.seasonalGuides, state.eventGuides)
  .map(entry => ({ ...entry, wear: collectionWear(entry) })));
export async function saveCollection(write: () => Promise<void>) {
  if (!activeSkyCharacter.value) { showToast('请先选择光遇角色', '收藏进度随角色保存'); return; }
  if (savingCollection.value) return;
  savingCollection.value = true;
  collectionError.value = '';
  try { await write(); return true; }
  catch (error) { collectionError.value = String(error); showToast('收藏保存失败，原记录已保留', String(error)); }
  finally { savingCollection.value = false; }
}
export function toggleCollectionOwned(entry: SkyCollectionEntry) {
  const id = activeSkyCharacter.value?.id || '';
  return saveCollection(() => setSkyCollectionOwned(entry, id, !entry.owned));
}
export function toggleCollectionWanted(entry: SkyCollectionEntry) {
  const id = activeSkyCharacter.value?.id || '';
  return saveCollection(() => setSkyCollectionWanted(entry, id, !entry.wanted));
}
