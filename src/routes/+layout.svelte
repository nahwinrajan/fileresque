<script lang="ts">
import '../app.css';
import { gsap } from 'gsap';
import { onMount } from 'svelte';

onMount(() => {
  // Respect reduced motion preference for all GSAP animations globally (WCAG 2.5.3)
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    gsap.globalTimeline.timeScale(0);
  }
});
</script>

<slot />
