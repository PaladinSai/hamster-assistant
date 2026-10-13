<script setup lang="ts">
import type { HexLiveDetail } from "./liveTypes";
import LiveRemain from "./LiveRemain.vue";
import { hexKindLabel } from "./liveUtils";

defineProps<{ detail: HexLiveDetail }>();
const emit = defineEmits<{ ended: [] }>();

function upgradeTip(titles: string[]): string | undefined {
  const lines = titles.map((t) => t.trim()).filter(Boolean);
  return lines.length ? lines.join("\n") : undefined;
}
</script>

<template>
  <div class="live-hex">
    <div class="live-hero">
      <div class="live-hero-text">
        <span class="live-kicker">六人组日历</span>
        <span class="live-hero-title">本季日程</span>
      </div>
      <div class="live-hero-meta">
        <LiveRemain :expiry="detail.expiry" @ended="emit('ended')" />
      </div>
    </div>
    <ul class="live-hex-days">
      <li
        v-for="(d, di) in detail.days"
        :key="di"
        class="live-hex-day"
      >
        <span class="live-badge soft">日 {{ d.day }}</span>
        <div class="live-hex-groups">
          <div
            v-for="(g, gi) in d.groups"
            :key="gi"
            class="live-hex-group"
            :class="g.kind"
          >
            <template v-if="g.kind === 'upgrade'">
              <span
                class="live-hex-chip tip upgrade-only"
                :data-tip="upgradeTip(g.choices.map((c) => c.title))"
              >增益</span>
            </template>
            <template v-else>
              <span class="live-hex-kind">{{ hexKindLabel(g.kind) }}</span>
              <div class="live-hex-choices">
                <template
                  v-for="(c, ci) in g.choices"
                  :key="ci"
                >
                  <span
                    v-if="ci > 0"
                    class="live-hex-or"
                  >或</span>
                  <span class="live-hex-choice">
                    <span
                      class="live-hex-chip"
                      :class="{ tip: g.kind !== 'challenge' && !!c.description }"
                      :data-tip="
                        g.kind !== 'challenge' && c.description
                          ? c.description
                          : undefined
                      "
                    >{{ c.title }}</span>
                    <span
                      v-if="g.kind === 'challenge' && c.description"
                      class="live-hex-desc"
                    >{{ c.description }}</span>
                  </span>
                </template>
              </div>
            </template>
          </div>
        </div>
      </li>
    </ul>
  </div>
</template>
