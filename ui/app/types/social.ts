import type { Branding, Theme } from '~/app.config'

export interface SocialLink { platform: string; url: string }
export interface GroupStyle { layout: 'list' | 'grid'; link_radius: string; icon_radius: string; spacing: string; title_align: 'left' | 'center' }
export const DEFAULT_GROUP_STYLE: GroupStyle = { layout: 'list', link_radius: '50%', icon_radius: '50%', spacing: '12px', title_align: 'left' }
export interface PublicProfile { username: string; display_name: string; bio: string; location: string; socials: SocialLink[]; avatar_url: string | null; cover_url: string | null }
export interface PublicLink { id: string; title: string; url: string; description: string; icon: string; icon_image: string | null; icon_font: string | null; click_count?: number; expires_at: string | null }
export interface PublicGroup { id: string; title: string; description: string; collapsible: boolean; style: GroupStyle; links: PublicLink[] }
export interface PublicProfileResponse { profile: PublicProfile; groups: PublicGroup[]; ungrouped: PublicLink[]; ungrouped_position: number; stats?: { views: number }; theme: Theme; branding: Branding }
export interface AdminGroup { id: string; title: string; description: string; collapsible: boolean; is_active: boolean; sort_order: number; style: GroupStyle }
export interface AdminLink extends PublicLink { group_id: string | null; is_active: boolean; sort_order: number }
