create or replace function public.is_super_admin()
returns boolean
language sql
security definer
stable
set search_path = public
as $$
  select coalesce((select is_super_admin from public.profiles where id = auth.uid() and actif = true), false)
$$;

create or replace function public.current_tenant_id()
returns uuid
language sql
security definer
stable
set search_path = public
as $$
  select tenant_id from public.profiles where id = auth.uid() and actif = true
$$;

create or replace function public.is_admin_tenant()
returns boolean
language sql
security definer
stable
set search_path = public
as $$
  select coalesce((select role = 'admin_tenant' from public.profiles where id = auth.uid() and actif = true), false)
$$;

create or replace function public.block_self_role_tenant_change()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  if old.id = auth.uid() and not public.is_super_admin() then
    if new.role is distinct from old.role then
      raise exception 'Vous ne pouvez pas modifier votre propre role';
    end if;
    if new.tenant_id is distinct from old.tenant_id then
      raise exception 'Vous ne pouvez pas modifier votre propre tenant';
    end if;
    if new.actif is distinct from old.actif then
      raise exception 'Vous ne pouvez pas modifier votre propre statut actif';
    end if;
  end if;
  return new;
end;
$$;

create or replace function public.check_commande_terrain_tenant_consistency()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  if new.magasin_id is not null and not exists (
    select 1 from public.magasins m where m.id = new.magasin_id and m.tenant_id = new.tenant_id
  ) then
    raise exception 'magasin_id n''appartient pas au tenant de la commande';
  end if;

  if new.commercial_id is not null and not exists (
    select 1 from public.profiles p where p.id = new.commercial_id and p.tenant_id = new.tenant_id
  ) then
    raise exception 'commercial_id n''appartient pas au tenant de la commande';
  end if;

  if new.valide_par is not null and not exists (
    select 1 from public.profiles p where p.id = new.valide_par and p.tenant_id = new.tenant_id
  ) then
    raise exception 'valide_par n''appartient pas au tenant de la commande';
  end if;

  if new.prepare_par is not null and not exists (
    select 1 from public.profiles p where p.id = new.prepare_par and p.tenant_id = new.tenant_id
  ) then
    raise exception 'prepare_par n''appartient pas au tenant de la commande';
  end if;

  return new;
end;
$$;

create trigger check_commande_terrain_tenant_consistency_trigger
  before insert or update on public.commande_terrain
  for each row
  execute function public.check_commande_terrain_tenant_consistency();

create or replace function public.check_magasin_tenant_consistency()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  if new.magasin_id is not null and not exists (
    select 1 from public.magasins m where m.id = new.magasin_id and m.tenant_id = new.tenant_id
  ) then
    raise exception 'magasin_id n''appartient pas à ce tenant';
  end if;
  return new;
end;
$$;

create trigger check_articles_cache_tenant_consistency_trigger
  before insert or update on public.articles_cache
  for each row
  execute function public.check_magasin_tenant_consistency();

create trigger check_evenements_business_tenant_consistency_trigger
  before insert or update on public.evenements_business
  for each row
  execute function public.check_magasin_tenant_consistency();

revoke execute on function public.check_commande_terrain_tenant_consistency() from public, anon, authenticated;
revoke execute on function public.check_magasin_tenant_consistency() from public, anon, authenticated;
