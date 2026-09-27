create index idx_profiles_tenant_id on public.profiles(tenant_id);

create index idx_magasins_tenant_id on public.magasins(tenant_id);

create index idx_articles_cache_tenant_id on public.articles_cache(tenant_id);
create index idx_articles_cache_magasin_id on public.articles_cache(magasin_id);

create index idx_commande_terrain_tenant_id on public.commande_terrain(tenant_id);
create index idx_commande_terrain_magasin_id on public.commande_terrain(magasin_id);
create index idx_commande_terrain_statut on public.commande_terrain(statut);

create index idx_commande_terrain_ligne_commande_id on public.commande_terrain_ligne(commande_id);

create index idx_commande_terrain_historique_commande_id on public.commande_terrain_historique(commande_id);

create index idx_evenements_business_tenant_id on public.evenements_business(tenant_id);
create index idx_evenements_business_magasin_id on public.evenements_business(magasin_id);
